use super::Envelope;
use super::Oscillator;
use super::SampleGenerator;
use super::SineOscillator;
use super::TriangleOscillator;
use super::SquareOscillator;

pub struct Voice {
    // Components
    osc: Box<dyn Oscillator + Send>,
    env: Box<Envelope>,

    // Modulators
    amp_modulators: Vec<Box<dyn SampleGenerator + Send>>,
    freq_modulators: Vec<Box<dyn SampleGenerator + Send>>,

    // Current state
    osc_freq: f32,
    osc_amp: f32,
    last_update: u64
}

impl Voice {
    pub fn new(sample_rate: u32) -> Self {
        //let osc = Box::new(SineOscillator::new(sample_rate));
        //let osc = Box::new(TriangleOscillator::new(sample_rate));
        let osc = Box::new(SquareOscillator::new(sample_rate));
        let env = Box::new(Envelope::new(sample_rate));
        let amp_modulators = Vec::new();
        let freq_modulators = Vec::new();
        let osc_freq = 440.0;
        let osc_amp = 0.5;
        let last_update = 0u64;
        let mut voice = Voice{osc, env, amp_modulators, freq_modulators, osc_freq, osc_amp, last_update};
        let mut modu = Box::new(SineOscillator::new(sample_rate));
        modu.set_freq(1.0);
        voice.add_freq_mod(modu);
        voice
    }

    pub fn get_sample(&mut self, sample_clock: u64) -> f32 {
        self.last_update = sample_clock;
        let freq_mod = self.get_freq_mod(sample_clock) * 20.0;
        let amp_mod = self.get_amp_mod(sample_clock);
        self.osc.set_freq(self.osc_freq + freq_mod);
        self.osc.get_sample(sample_clock) * (self.osc_amp + amp_mod) * self.env.get_sample(sample_clock)
    }

    fn get_freq_mod(&self, sample_clock: u64) -> f32 {
        let mut freq_mod = 0.0;
        for fm in self.freq_modulators.iter() {
            freq_mod += fm.get_sample(sample_clock) * 5.0;
        }
        freq_mod
    }

    fn get_amp_mod(&self, sample_clock: u64) -> f32 {
        let mut amp_mod = 0.0;
        for am in self.amp_modulators.iter() {
            amp_mod += am.get_sample(sample_clock);
        }
        amp_mod
    }

    pub fn set_oscillator(&mut self, osc: Box<dyn Oscillator + Send>) {
        self.osc = osc;
    }

    pub fn add_freq_mod(&mut self, fm: Box<dyn SampleGenerator + Send>) {
        self.freq_modulators.push(fm);
    }

    pub fn add_amp_mod(&mut self, am: Box<dyn SampleGenerator + Send>) {
        self.amp_modulators.push(am);
    }

    pub fn trigger(&mut self) {
        self.env.trigger(self.last_update);
    }

    pub fn release(&mut self) {
        self.env.release(self.last_update);
    }
}
