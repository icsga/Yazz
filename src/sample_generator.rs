use super::synth::SoundData;

pub trait SampleGenerator {
    fn get_sample(&mut self, frequency: f32, sample_clock: i64, data: &SoundData) -> f32;
}

