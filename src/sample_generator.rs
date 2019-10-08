pub trait SampleGenerator {
    fn get_sample(&mut self, frequency: f32, sample_clock: u64) -> f32;
}

