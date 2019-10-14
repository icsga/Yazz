use super::sound::SoundData;

pub trait SampleGenerator {
    fn get_sample(&mut self, frequency: f32, sample_clock: i64, data: &SoundData, reset: bool) -> (f32, bool);
    fn reset(&mut self);
}

