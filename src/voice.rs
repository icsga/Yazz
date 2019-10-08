use super::Envelope;
use super::Oscillator;
use super::SampleGenerator;
use super::MultiOscillator;
//use super::SineOscillator;
//use super::TriangleOscillator;
//use super::SquareOscillator;

pub struct Voice {
    // Components
    //osc: Box<dyn SampleGenerator + Send>,
    osc: MultiOscillator,
    env: Box<Envelope>,

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
        //let osc = Box::new(SineOscillator::new(sample_rate));
        //let osc = Box::new(TriangleOscillator::new(sample_rate));
        //let osc = Box::new(MultiOscillator::new(sample_rate));
        let mut osc = MultiOscillator::new(sample_rate);
        osc.set_voice_num(3);
        let env = Box::new(Envelope::new(sample_rate));
        let amp_modulators = Vec::new();
        let freq_modulators = Vec::new();
        let input_freq = 440.0;
        let osc_amp = 0.5;
        let last_update = 0u64;
        let mut voice = Voice{osc, env, amp_modulators, freq_modulators, input_freq, osc_amp, last_update};
        let mut modu = Box::new(MultiOscillator::new(sample_rate));
        modu.set_ratios(0.0, 1.0, 0.0, 0.0);
        voice.add_freq_mod(modu);
        voice
    }

    pub fn get_sample(&mut self, sample_clock: u64) -> f32 {
        self.last_update = sample_clock;
        let wave_mod = (self.get_freq_mod(sample_clock) + 1.0) * 1.5;
        self.osc.set_ratio(wave_mod);
        //self.osc.set_ratios(0.7, 0.0, 0.3, 0.0);
        let freq_mod = 0.0;
        let amp_mod = self.get_amp_mod(sample_clock);
        self.osc.get_sample(self.input_freq + freq_mod, sample_clock) * (self.osc_amp + amp_mod) * self.env.get_sample(sample_clock)
    }

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
        self.env.trigger(self.last_update);
    }

    pub fn release(&mut self) {
        self.env.release(self.last_update);
    }
    
    pub fn set_wave_ratio(&mut self, value: usize) {
        match value {
            0 => self.osc.set_ratios(1.0, 0.0, 0.0, 0.0),
            1 => self.osc.set_ratios(0.0, 1.0, 0.0, 0.0),
            2 => self.osc.set_ratios(0.0, 0.0, 1.0, 0.0),
            3 => self.osc.set_ratios(0.0, 0.0, 0.0, 1.0),
            _ => {}
        }
    }
}
