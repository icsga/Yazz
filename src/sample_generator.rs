pub trait SampleGenerator {
    fn get_sample(&self, sample_clock: u64) -> f32;
}

