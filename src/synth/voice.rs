use super::Envelope;
use super::Filter;
use super::Float;
use super::Lfo;
use super::Modulator;
use super::Oscillator;
use super::{Parameter, ParameterValue, SynthParam};
use super::SampleGenerator;
use super::MultiOscillator;
use super::SoundData;

use std::sync::Arc;

use log::{info, trace, warn};

pub const NUM_OSCILLATORS: usize = 3;
pub const NUM_ENVELOPES: usize = 2;
pub const NUM_FILTERS: usize = 2;
pub const NUM_LFOS: usize = 2;

pub struct Voice {
    // Components
    //osc: Box<dyn SampleGenerator + Send>,
    osc: [MultiOscillator; NUM_OSCILLATORS],
    env: [Envelope; NUM_ENVELOPES],
    pub filter: [Filter; NUM_FILTERS],
    lfo: [Lfo; NUM_LFOS],

    // Current state
    triggered: bool,
    pub trigger_seq: u64,
    pub key: u8,          // Key that was pressed to trigger this voice
    velocity: u8,         // Velocity of NoteOn event
    input_freq: Float,    // Frequency to play as received from Synth
    osc_amp: Float,
    last_update: i64,
}

impl Voice {
    pub fn new(sample_rate: u32) -> Self {
        let osc = [
            MultiOscillator::new(sample_rate, 0),
            MultiOscillator::new(sample_rate, 1),
            MultiOscillator::new(sample_rate, 2),
        ];
        let env = [
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
        let velocity = 0;
        let input_freq = 440.0;
        let osc_amp = 0.5;
        let last_update = 0i64;
        let voice = Voice{osc, env, filter, lfo, triggered, trigger_seq, key, velocity, input_freq, osc_amp, last_update};
        voice
    }

    pub fn get_sample(&mut self, sample_clock: i64, modulators: &[Modulator], sound: &SoundData, sound_global: &SoundData, sound_local: &mut SoundData) -> Float {
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
        // =========================
        for m in sound.modul.iter() {

            if !m.active {
                continue;
            }

            // Get modulator source output
            let mod_val: Float = match m.source_func {
                Parameter::Lfo => {
                    let (val, reset) = self.lfo[m.source_func_id - 1].get_sample(sample_clock, &sound_local.lfo[m.source_func_id - 1], false);
                    val
                },
                Parameter::Envelope => {
                    self.env[m.source_func_id - 1].get_sample(sample_clock, &sound_local.env[m.source_func_id - 1])
                }
                _ => 0.0, // TODO: This also sets non-global vars, optimize that
            } * m.scale;

            // Get current value of target parameter
            let param = SynthParam{function: m.target_func, function_id: m.target_func_id, parameter: m.target_param, value: ParameterValue::NoValue};
            let current_val = sound_global.get_value(&param).clone();
            let mut val = match current_val {
                ParameterValue::Int(x) => x as Float,
                ParameterValue::Float(x) => x,
                _ => panic!()
            };

            // Update value if mod source is local
            if !m.is_global {
                val += mod_val;
                info!("val={}, mod_val={}", val, mod_val);
            }

            // Update parameter in voice sound data
            let param = SynthParam{function: m.target_func, function_id: m.target_func_id, parameter: m.target_param, value: ParameterValue::Float(val)};
            sound_local.set_parameter(&param);
        }

        // Get mixed output from oscillators
        for (i, osc) in self.osc.iter_mut().enumerate() {
            if sound_local.osc[i].key_follow == 0 {
                freq = 440.0;
            } else {
                freq = self.input_freq;
            }
            freq *= sound_local.osc[i].freq_offset;
            let (sample, wave_complete) = osc.get_sample(freq, sample_clock, sound_local, reset);
            result += sample * (self.osc_amp + amp_mod);
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
        //result = self.filter[0].process(result, sample_clock, &sound_local.filter[0]);

        // Apply the volume envelope
        result *= self.env[0].get_sample(sample_clock, &sound_local.env[0]);
        if result > 1.0 {
            panic!("Voice: {}", result);
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
        self.velocity = velocity;
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
