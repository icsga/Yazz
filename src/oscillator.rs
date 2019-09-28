pub trait Oscillator {
    fn set_freq(&mut self, freq: f32);
    fn get_freq(&self) -> f32;
    fn get_sample(&self, sample_clock: u64, freq: f32) -> f32;
    fn get_sample_rate(&self) -> u32;
}

