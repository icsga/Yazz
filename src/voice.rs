use super::Oscillator;
use super::ComplexSineOscillator;

pub struct Voice {
    osc: Box<dyn Oscillator + Send>,
    amp_modulators: Vec<Box<dyn Oscillator + Send>>,
    freq_modulators: Vec<Box<dyn Oscillator + Send>>,
    osc_freq: f32,
    osc_amp: f32
}

impl Voice {
    pub fn new(sample_rate: u32) -> Self {
        let osc = Box::new(ComplexSineOscillator::new(sample_rate));
        let amp_modulators = Vec::new();
        let freq_modulators = Vec::new();
        let osc_freq = 440.0;
        let osc_amp = 0.1;
        Voice{osc, amp_modulators, freq_modulators, osc_freq, osc_amp}
    }

    pub fn get_sample(&self, sample_clock: u64) -> f32 {
        let freq_mod = self.get_freq_mod(sample_clock) * 20.0;
        let amp_mod = self.get_amp_mod(sample_clock);
        self.osc.get_sample(sample_clock, self.osc_freq + freq_mod) * (self.osc_amp + amp_mod)
    }

    fn get_freq_mod(&self, sample_clock: u64) -> f32 {
        let mut freq_mod = 0.0;
        for fm in self.freq_modulators.iter() {
            freq_mod += fm.get_sample(sample_clock, fm.get_freq());
        }
        freq_mod
    }

    fn get_amp_mod(&self, sample_clock: u64) -> f32 {
        let mut amp_mod = 0.0;
        for am in self.amp_modulators.iter() {
            amp_mod += am.get_sample(sample_clock, am.get_freq());
        }
        amp_mod
    }

    pub fn set_oscillator(&mut self, osc: Box<dyn Oscillator + Send>) {
        self.osc = osc;
    }

    pub fn add_freq_mod(&mut self, fm: Box<dyn Oscillator + Send>) {
        self.freq_modulators.push(fm);
    }

    pub fn add_amp_mod(&mut self, am: Box<dyn Oscillator + Send>) {
        self.amp_modulators.push(am);
    }
}
