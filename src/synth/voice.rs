use super::Envelope;
use super::Filter;
use super::Float;
use super::Lfo;
use super::{Parameter, ParamId, SynthParam, MenuItem};
use super::{PlayMode, FilterRouting};
use super::SynthState;
use super::{Oscillator, OscData};
use super::SoundData;

use log::info;
use wavetable::{Wavetable, WavetableRef};

use std::sync::Arc;

pub const NUM_OSCILLATORS: usize = 3;
pub const NUM_ENVELOPES: usize = 3;
pub const NUM_FILTERS: usize = 2;
pub const NUM_LFOS: usize = 2;

pub struct Voice {
    // Components
    osc: [Oscillator; NUM_OSCILLATORS],
    env: [Envelope; NUM_ENVELOPES],
    pub filter: [Filter; NUM_FILTERS],
    lfo: [Lfo; NUM_LFOS],

    // Current state
    triggered: bool,
    pub trigger_seq: u64, // Sequence number for keeping track of trigger order
    pub key: u8,          // Key that was pressed to trigger this voice
    velocity: Float,      // Raw velocity of NoteOn event (for use as modulation source)
    scaled_vel: Float,    // Velocity scaled according to sound settings (for use as amplifier)
    input_freq: Float,    // Frequency to play as received from Synth
    last_update: i64,
}

impl Voice {
    pub fn new(sample_rate: u32, default_wavetable: Arc<Wavetable>) -> Self {
        let osc = [
            Oscillator::new(sample_rate, Arc::clone(&default_wavetable)),
            Oscillator::new(sample_rate, Arc::clone(&default_wavetable)),
            Oscillator::new(sample_rate, Arc::clone(&default_wavetable)),
        ];
        let env = [
            Envelope::new(sample_rate as Float),
            Envelope::new(sample_rate as Float),
            Envelope::new(sample_rate as Float),
        ];
        let filter = [
            Filter::new(sample_rate),
            Filter::new(sample_rate),
        ];
        let lfo = [
            Lfo::new(sample_rate),
            Lfo::new(sample_rate),
        ];
        let voice = Voice{
                osc,
                env,
                filter,
                lfo,
                triggered: false,
                trigger_seq: 0,
                key: 0,
                velocity: 0.0,
                scaled_vel: 0.0,
                input_freq: 440.0,
                last_update: 0i64};
        voice
    }

    pub fn reset(&mut self) {
        self.triggered = false;
        for e in &mut self.env {
            e.reset();
        }
        for f in &mut self.filter {
            f.reset();
        }
    }

    fn get_frequency(data: &OscData, input_freq: Float) -> Float {
        let mut freq = if data.key_follow == 0 {
            440.0
        } else {
            input_freq
        };
        freq *= data.freq_offset;
        freq
    }

    fn get_mod_values(&mut self, sample_clock: i64, sound_global: &SoundData, sound_local: &mut SoundData) {
        // Get modulated values from global sound and discard values that were
        // modulated for the previous sample. Complete copy is faster than
        // looping over the modulators.
        *sound_local = *sound_global;

        // Then update the local sound with mod values
        let mut param_id = ParamId{..Default::default()};
        let mut synth_param = SynthParam{..Default::default()};
        for m in sound_global.modul.iter() {

            if !m.active {
                continue;
            }

            // Get current value of target parameter
            param_id.set(m.target_func, m.target_func_id, m.target_param);
            let mut current_val = sound_local.get_value(&param_id);

            if !m.is_global {

                // Get modulator source output
                let mod_val: Float = match m.source_func {
                    Parameter::Oscillator => {
                        let id = m.source_func_id - 1;
                        let freq = Voice::get_frequency(&sound_local.osc[id], self.input_freq);
                        let (val, _) = self.osc[id].get_sample(freq, sample_clock, &sound_local.osc[id], false);
                        val
                    },
                    Parameter::Lfo => {
                        let (val, _) = self.lfo[m.source_func_id - 1].get_sample(sample_clock, &sound_local.lfo[m.source_func_id - 1], false);
                        val
                    },
                    Parameter::Envelope => {
                        self.env[m.source_func_id - 1].get_sample(sample_clock, &sound_local.env[m.source_func_id - 1])
                    }
                    Parameter::Velocity => {
                        self.velocity
                    }
                    _ => 0.0,
                } * m.scale;

                let mut val = current_val.as_float();

                // Update value
                let dest_range = MenuItem::get_val_range(param_id.function, param_id.parameter);
                val = dest_range.safe_add(val, mod_val);

                // Update parameter in voice sound data
                current_val.set_from_float(val);
            }

            // TODO: Too much copying
            synth_param.set(m.target_func, m.target_func_id, m.target_param, current_val);
            sound_local.set_parameter(&synth_param);
        }
    }

    pub fn get_sample(&mut self,
                      sample_clock: i64,
                      sound_global: &SoundData,
                      sound_local: &mut SoundData,
                      global_state: &SynthState) -> Float {
        if !self.is_running() {
            return 0.0;
        }
        let mut input_f1 = 0.0;
        let mut input_f2 = 0.0;
        let mut result_direct = 0.0;
        self.last_update = sample_clock;
        let mut reset = false;
        let input_freq = self.input_freq * global_state.freq_factor;
        let mut freq: Float;

        // Prepare modulation values
        self.get_mod_values(sample_clock, sound_global, sound_local);

        // Get mixed output from oscillators
        for (i, osc) in self.osc.iter_mut().enumerate() {
            freq = Voice::get_frequency(&sound_local.osc[i], input_freq);
            let (sample, wave_complete) = osc.get_sample(freq, sample_clock, &sound_local.osc[i], reset);
            let sample_amped = sample * sound_local.osc[i].level * self.scaled_vel;
            input_f1 += sample_amped * osc.filter1_out;
            input_f2 += sample_amped * osc.filter2_out;
            result_direct += sample_amped * osc.direct_out;
            // Sync oscillator 1 to 0
            reset = i == 0 && wave_complete && sound_local.osc[1].sync == 1;
        }

        // Feed it into the filters
        let mut result: Float = self.apply_filter(sample_clock,
                                                  sound_local,
                                                  input_f1,
                                                  input_f2,
                                                  input_freq);
        result += result_direct;

        // Apply the volume envelope
        let env_amp = self.env[0].get_sample(sample_clock, &sound_local.env[0]);
        if sound_local.patch.env_depth > 0.0 {
            result *= env_amp * sound_local.patch.env_depth;
        }
        if result > 1.0 {
            //panic!("Voice: {}", result);
            result = 1.0;
        } else if result < -1.0 {
            result = -1.0;
        }

        result
    }

    pub fn apply_filter(&mut self,
                        sample_clock: i64,
                        sound_local: &mut SoundData,
                        input_f1: Float,
                        mut input_f2: Float,
                        input_freq: Float) -> Float {
        let filter_env = self.env[1].get_sample(sample_clock, &sound_local.env[1]); // Env2 is normaled to filter cutoff
        let output_f1  = self.filter[0].process(input_f1, &mut sound_local.filter[0], input_freq, filter_env);
        let mut result = match sound_local.patch.filter_routing {
            FilterRouting::Parallel => {
                output_f1
            }
            FilterRouting::Serial => {
                input_f2 += output_f1;
                0.0
            }
        };
        result += self.filter[1].process(input_f2, &mut sound_local.filter[1], input_freq, filter_env);
        result
    }

    pub fn set_key(&mut self, key: u8) {
        self.key = key;
    }

    pub fn set_freq(&mut self, freq: Float) {
        self.input_freq = freq;
    }

    pub fn set_velocity(&mut self, velocity: u8, sensitivity: Float) {
        self.velocity = velocity as Float / 127.0;
        // Sensitivity gives us the range of velocity values from the maximum.
        // Sens 0.7 means scaled_velocity ranges from 0.3 - 1.0.
        self.scaled_vel = (1.0 - sensitivity) + (self.velocity * sensitivity);
    }

    pub fn set_wavetable(&mut self, osc_id: usize, wt: WavetableRef) {
        self.osc[osc_id].set_wavetable(wt);
    }

    pub fn update_routing(&mut self, osc_id: usize, osc_data: &OscData) {
        self.osc[osc_id].update_routing(osc_data);
    }

    pub fn trigger(&mut self, trigger_seq: u64, trigger_time: i64, sound: &SoundData) {
        let trigger = match sound.patch.play_mode {
            PlayMode::Poly => true, // Poly: Always retrigger
            PlayMode::Mono => true, // Mono: Always retrigger
            PlayMode::Legato => {   // Legator: Only retrigger if not already triggered
                !self.triggered
            }
        };
        self.trigger_seq = trigger_seq;
        if trigger {
            if !self.is_running() {
                for osc in self.osc.iter_mut() {
                    osc.reset(trigger_time);
                }
            }
            for i in 0..NUM_ENVELOPES {
                self.env[i].trigger(trigger_time, &sound.env[i]);
            }
            for (i, lfo) in self.lfo.iter_mut().enumerate() {
                lfo.reset(trigger_time, sound.lfo[i].phase);
            }
        }
        self.triggered = true;
    }

    // TODO: Release velocity
    pub fn key_release(&mut self, _velocity: u8, pedal_held: bool, sound: &SoundData) {
        self.triggered = false;
        if !pedal_held {
            self.release_envelopes(sound);
        }
    }

    pub fn pedal_release(&mut self, sound: &SoundData) {
        if !self.triggered {
            self.release_envelopes(sound);
        }
    }

    pub fn is_triggered(&self) -> bool {
        self.triggered
    }

    pub fn is_running(&self) -> bool {
        self.triggered || self.env[0].is_running()
    }

    fn release_envelopes(&mut self, sound: &SoundData) {
        for i in 0..NUM_ENVELOPES {
            self.env[i].release(self.last_update, &sound.env[i]);
        }
    }
}
