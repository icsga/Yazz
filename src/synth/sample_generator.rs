use super::Float;
use super::SoundData;

pub trait SampleGenerator {
    fn get_sample(&mut self, frequency: Float, sample_clock: i64, data: &SoundData, reset: bool) -> (Float, bool);
    fn reset(&mut self, sample_clock: i64);
}

