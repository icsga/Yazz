use crate::Float;

/* Taken from https://www.earlevel.com/main/2012/12/15/a-one-pole-filter/ */
pub struct OnePole {
    sample_rate: Float,
    a0: Float,
    b1: Float,
    z1: Float
}

impl OnePole {
    pub fn new(sample_rate: u32) -> OnePole {
        OnePole{sample_rate: sample_rate as Float, a0: 0.0, b1: 0.0, z1: 0.0}
    }

    pub fn update(&mut self, cutoff: Float) {
        let freq = cutoff / self.sample_rate;
        self.b1 = (-2.0 * std::f64::consts::PI * freq).exp();
        self.a0 = 1.0 - self.b1;
    }

    pub fn process(&mut self, sample: Float) -> Float {
        self.z1 = sample * self.a0 + self.z1 * self.b1;
        self.z1
    }
}
