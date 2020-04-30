use super::Envelope;
use super::Filter;
use super::Float;
use super::Lfo;
use super::{Parameter, ParameterValue, ParamId, SynthParam, MenuItem};
use super::SampleGenerator;
use super::{WtOsc, WtOscData, Wavetable, WavetableRef};
use super::SoundData;

use std::sync::Arc;

use log::{info, trace, warn};

pub const NUM_OSCILLATORS: usize = 3;
pub const NUM_ENVELOPES: usize = 3;
pub const NUM_FILTERS: usize = 2;
pub const NUM_LFOS: usize = 2;

pub struct Voice {
    // Components
    osc: [WtOsc; NUM_OSCILLATORS],
    env: [Envelope; NUM_ENVELOPES],
    pub filter: [Filter; NUM_FILTERS],
    lfo: [Lfo; NUM_LFOS],

    // Current state
    triggered: bool,
    pub trigger_seq: u64,
    pub key: u8,          // Key that was pressed to trigger this voice
    velocity: Float,      // Velocity of NoteOn event
    input_freq: Float,    // Frequency to play as received from Synth
    osc_amp: Float,
    last_update: i64,
}

impl Voice {
    pub fn new(sample_rate: u32, default_wavetable: Arc<Wavetable>) -> Self {
        let osc = [
            WtOsc::new(sample_rate, 0, Arc::clone(&default_wavetable)),
            WtOsc::new(sample_rate, 1, Arc::clone(&default_wavetable)),
            WtOsc::new(sample_rate, 2, Arc::clone(&default_wavetable)),
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
        let triggered = false;
        let trigger_seq = 0;
        let key = 0;
        let velocity = 0.0;
        let input_freq = 440.0;
        let osc_amp = 0.5;
        let last_update = 0i64;
        let voice = Voice{
                osc,
                env,
                filter,
                lfo,
                triggered,
                trigger_seq,
                key,
                velocity,
                input_freq,
                osc_amp,
                last_update};
        voice
    }

    fn get_frequency(data: &WtOscData, input_freq: Float) -> Float {
        let mut freq: Float = if data.key_follow == 0 {
            440.0
        } else {
            input_freq
        };
        freq *= data.freq_offset;
        freq
    }

    fn get_mod_values(&mut self, sample_clock: i64, sound: &SoundData, sound_global: &SoundData, sound_local: &mut SoundData) {
        // Get modulated values from global sound and discard values that were
        // modulated for the previous sample. Complete copy is faster than
        // looping over the modulators.
        *sound_local = *sound_global;

        // Then update the local sound with mod values
        for m in sound.modul.iter() {

            if !m.active {
                continue;
            }

            // Get current value of target parameter
            let param = ParamId{function: m.target_func, function_id: m.target_func_id, parameter: m.target_param};
            let mut current_val = sound_local.get_value(&param);

            if !m.is_global {

                // Get modulator source output
                let mod_val: Float = match m.source_func {
                    Parameter::Oscillator => {
                        let id = m.source_func_id - 1;
                        let freq = Voice::get_frequency(&sound_local.osc[id], self.input_freq);
                        let (val, wave_complete) = self.osc[id].get_sample(freq, sample_clock, sound_local, false);
                        val
                    },
                    Parameter::Lfo => {
                        let (val, reset) = self.lfo[m.source_func_id - 1].get_sample(sample_clock, &sound_local.lfo[m.source_func_id - 1], false);
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
                let dest_range = MenuItem::get_val_range(param.function, param.parameter);
                val = dest_range.safe_add(val, mod_val);

                // Update parameter in voice sound data
                current_val.set_from_float(val);
            }

            let param = SynthParam{function: m.target_func, function_id: m.target_func_id, parameter: m.target_param, value: current_val};
            sound_local.set_parameter(&param);
        }
    }

    pub fn get_sample(&mut self, sample_clock: i64, sound: &SoundData, sound_global: &SoundData, sound_local: &mut SoundData) -> Float {
        if !self.is_running() {
            return 0.0;
        }
        let mut result = 0.0;
        //let amp_mod = self.get_amp_mod(sample_clock);
        let amp_mod = 0.0;
        self.last_update = sample_clock;
        let mut reset = false;
        let mut freq: Float;

        // Prepare modulation values
        self.get_mod_values(sample_clock, sound, sound_global, sound_local);

        // Get mixed output from oscillators
        for (i, osc) in self.osc.iter_mut().enumerate() {
            freq = Voice::get_frequency(&sound_local.osc[i], self.input_freq);
            let (sample, wave_complete) = osc.get_sample(freq, sample_clock, sound_local, reset);
            result += sample * sound_local.osc[i].level * (self.osc_amp + amp_mod);
            if i == 0 && wave_complete && sound_local.osc[1].sync == 1 {
                reset = true; // Sync next oscillator in list (osc 1)
            } else {
                reset = false;
            }
        }
        let level_sum = sound_local.osc[0].level + sound_local.osc[1].level + sound_local.osc[2].level;
        if level_sum > 1.0 {
            // Normalize level to avoid distortion
            result /= level_sum;
        }

        // Feed it into the filter
        // TODO: Use both filters, use different filter routings
        result = self.filter[0].process(result, sample_clock, &mut sound_local.filter[0]);

        // Apply the volume envelope
        result *= self.env[0].get_sample(sample_clock, &sound_local.env[0]);
        if result > 1.0 {
            //panic!("Voice: {}", result);
            result = 1.0;
        }

        result
    }

    pub fn set_key(&mut self, key: u8) {
        self.key = key;
    }

    pub fn set_freq(&mut self, freq: Float) {
        self.input_freq = freq;
    }

    pub fn set_velocity(&mut self, velocity: u8) {
        self.velocity = velocity as Float;
    }

    pub fn set_wavetable(&mut self, osc_id: usize, wt: WavetableRef) {
        self.osc[osc_id].set_wavetable(wt);
    }

    pub fn trigger(&mut self, trigger_seq: u64, trigger_time: i64, sound: &SoundData) {
        self.triggered = true;
        self.trigger_seq = trigger_seq;
        for i in 0..NUM_ENVELOPES {
            self.env[i].trigger(trigger_time, &sound.env[i]);
        }
        for osc in self.osc.iter_mut() {
            osc.reset(trigger_time);
        }
        for lfo in self.lfo.iter_mut() {
            lfo.reset(trigger_time);
        }
    }

    // TODO: Release velocity
    pub fn release(&mut self, velocity: u8, sound: &SoundData) {
        self.triggered = false;
        for i in 0..NUM_ENVELOPES {
            self.env[i].release(self.last_update, &sound.env[i]);
        }
    }

    pub fn is_triggered(&self) -> bool {
        self.triggered
    }

    pub fn is_running(&self) -> bool {
        self.triggered || self.env[0].is_running()
    }
}
