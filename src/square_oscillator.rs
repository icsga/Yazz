use super::oscillator::Oscillator;

pub struct SquareWaveOscillator {
    freq: f32,
    sample_rate: u32,
    phase: f32
}

impl SquareWaveOscillator {
    pub fn new(sample_rate: u32) -> Self {
        let freq = 440.0;
        let phase = 0.5;
        let osc = SquareWaveOscillator{freq, sample_rate, phase};
        osc
    }
}

impl Oscillator for SquareWaveOscillator {
    fn set_freq(&mut self, freq: f32) {
        self.freq = freq;
    }

    fn get_freq(&self) -> f32 {
        self.freq
    }

    fn get_sample(&self, sample_clock: u64, freq: f32) -> f32 {
        let range = self.sample_rate as f32 / freq;
        let clock_offset: f32 = (sample_clock % range as u64) as f32;
        //println!("co: {}, range: {}", clock_offset, range);
        if clock_offset > (range / 2.0) {
            0.0
        } else {
            1.0
        }
    }

    fn get_sample_rate(&self) -> u32 {
        self.sample_rate
    }
}

