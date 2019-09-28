use super::oscillator::Oscillator;
use std::cell::RefCell;

pub struct SineOscillator {
    freq: f32,
    sample_rate: u32,
    last_update: RefCell<(u64, f32)>
}

impl SineOscillator {
    pub fn new(sample_rate: u32) -> SineOscillator {
        let osc = SineOscillator{freq: 440.0, sample_rate: sample_rate, last_update: RefCell::new((0, 0.0))};
        osc
    }
}

impl Oscillator for SineOscillator {
    fn set_freq(&mut self, freq: f32) {
        self.freq = freq;
    }

    fn get_freq(&self) -> f32 {
        self.freq
    }

    fn get_sample(&self, sample_clock: u64, freq: f32) -> f32 {
        if sample_clock != self.last_update.borrow().0 {
            self.last_update.borrow_mut().0 = sample_clock;
            self.last_update.borrow_mut().1 = (sample_clock as f32 * freq * 2.0 * 3.141592 / self.sample_rate as f32).sin();
        }
        self.last_update.borrow().1
    }

    fn get_sample_rate(&self) -> u32 {
        self.sample_rate
    }
}

