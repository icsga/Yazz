use super::Envelope;
use super::Oscillator;
use super::SampleGenerator;
use super::MultiOscillator;
use super::SoundData;

use std::sync::Arc;

pub struct Voice {
    // Components
    //osc: Box<dyn SampleGenerator + Send>,
    osc: [MultiOscillator; 3],
    env: [Envelope; 2],

    // Modulators
    amp_modulators: Vec<Box<dyn SampleGenerator + Send>>,
    freq_modulators: Vec<Box<dyn SampleGenerator + Send>>,

    // Current state
    input_freq: f32, // Frequency to play as received from Synth
    osc_amp: f32,
    last_update: u64
}

impl Voice {
    pub fn new(sample_rate: u32) -> Self {
        let osc = [
            MultiOscillator::new(sample_rate, 0),
            MultiOscillator::new(sample_rate, 1),
            MultiOscillator::new(sample_rate, 2),
        ];
        //osc.set_voice_num(3);
        //osc.set_ratios(0.0, 0.0, 0.0, 1.0, 0.0);
        //osc.set_ratios(sound.osc[0].mix[0], sound.osc[0].mix[1], sound.osc[0].mix[2], sound.osc[0].mix[3], sound.osc[0].mix[4]);
        let env = [
            Envelope::new(sample_rate as f32, 0),
            Envelope::new(sample_rate as f32, 1),
        ];
        let amp_modulators = Vec::new();
        let freq_modulators = Vec::new();
        let input_freq = 440.0;
        let osc_amp = 0.5;
        let last_update = 0u64;
        let voice = Voice{osc, env, amp_modulators, freq_modulators, input_freq, osc_amp, last_update};
        //let mut modu = Box::new(MultiOscillator::new(sample_rate));
        //modu.set_ratios(0.0, 1.0, 0.0, 0.0, 0.0);
        //voice.add_freq_mod(modu);
        voice
    }

    pub fn get_sample(&mut self, sample_clock: u64, sound: &SoundData) -> f32 {
        let mut result = 0.0;
        //let amp_mod = self.get_amp_mod(sample_clock);
        let amp_mod = 0.0;
        let freq_mod = 0.0;
        self.last_update = sample_clock;
        for (i, osc) in self.osc.iter_mut().enumerate() {
            let freq = (self.input_freq + freq_mod) * sound.osc[i].freq_offset;
            result += osc.get_sample(freq, sample_clock, sound) * (self.osc_amp + amp_mod);
        }
        result /= sound.osc[0].level + sound.osc[1].level + sound.osc[2].level;
        result *= self.env[0].get_sample(sample_clock, sound);
        if result > 1.0 {
            panic!("Voice: {}", result);
        }
        result
    }

    /*
    fn get_freq_mod(&mut self, sample_clock: u64) -> f32 {
        let mut freq_mod = 0.0;
        for fm in self.freq_modulators.iter_mut() {
            freq_mod += fm.get_sample(0.25, sample_clock) * 1.0;
        }
        freq_mod
    }

    fn get_amp_mod(&mut self, sample_clock: u64) -> f32 {
        let mut amp_mod = 0.0;
        for am in self.amp_modulators.iter_mut() {
            amp_mod += am.get_sample(1.0, sample_clock);
        }
        amp_mod
    }
    */

    /*
    pub fn set_oscillator(&mut self, osc: Box<dyn SampleGenerator + Send>) {
        self.osc = osc;
    }
    */

    pub fn add_freq_mod(&mut self, fm: Box<dyn SampleGenerator + Send>) {
        self.freq_modulators.push(fm);
    }

    pub fn add_amp_mod(&mut self, am: Box<dyn SampleGenerator + Send>) {
        self.amp_modulators.push(am);
    }

    pub fn set_freq(&mut self, freq: f32) {
        self.input_freq = freq;
    }

    pub fn trigger(&mut self) {
        self.env[0].trigger(self.last_update);
    }

    pub fn release(&mut self) {
        self.env[0].release(self.last_update);
    }
}
