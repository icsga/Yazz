use super::SampleGenerator;

pub struct TriangleOscillator {
    sample_rate: u32,
    last_update: u64, // Time of last sample generation
    last_pos: f32,
}

impl TriangleOscillator {
    pub fn new(sample_rate: u32) -> TriangleOscillator {
        let last_update = 0;
        let last_pos = 0.0;
        let osc = TriangleOscillator{sample_rate, last_update, last_pos};
        osc
    }
}

impl SampleGenerator for TriangleOscillator {
    fn get_sample(&self, frequency: f32, sample_clock: u64) -> f32 {
        let dt = sample_clock - self.last_update;

        let freq_speed = frequency / self.sample_rate as f32;
        let diff = freq_speed * dt as f32;

        self.last_pos += diff;
        if self.last_pos > 1.0 {
            self.last_pos -= 1.0;
        }

        // advance time
        self.last_update += dt;

        if self.last_pos < 0.25 {
            self.last_pos / 0.25
        } else if self.last_pos < 0.5 {
            1.0 + (1.0 - self.last_pos / 0.25)
        } else if self.last_pos < 0.75 {
            (2.0 - self.last_pos / 0.25)
        } else {
            -1.0 - (3.0 - self.last_pos / 0.25)
        }
    }
}

