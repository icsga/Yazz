use super::SampleGenerator;

pub trait Oscillator: SampleGenerator {
    fn set_freq(&mut self, freq: f32);
    fn get_freq(&self) -> f32;
}

