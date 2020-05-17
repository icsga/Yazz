// Adapted from SC3's ResonZ

use crate::Float;
use super::{Filter, FilterData};

pub struct ResonZ {
    sample_rate: Float,
    radians_per_sample: Float,
    reso_factor: Float,
    reso_offset: Float,

    y1: Float,
    y2: Float,
    a0: Float,
    b1: Float,
    b2: Float,
}

impl ResonZ {
    pub fn new(sample_rate: Float) -> ResonZ {
        ResonZ{sample_rate: sample_rate,
              radians_per_sample: (std::f64::consts::PI * 2.0) / sample_rate,
              reso_factor: 1.0 / (5.0 - 1.0),
              reso_offset: 1.0,
              y1: 0.0, y2: 0.0, a0: 0.0, b1: 0.0, b2: 0.0,}
    }

    pub fn reset(&mut self) {
        self.y1 = 0.0;
        self.y2 = 0.0;
        self.a0 = 0.0;
        self.b1 = 0.0;
        self.b2 = 0.0;
    }

    pub fn process(&mut self, sample: Float) -> Float {
        let y0 = sample + self.b1 * self.y1 + self.b2 * self.y2;
        let result = self.a0 * (y0 - self.y2);
        self.y2 = Filter::normalize(self.y1);
        self.y1 = Filter::normalize(y0);
        result
    }

    pub fn update(&mut self, data: &FilterData, freq: Float) {
        let resonance = data.resonance * self.reso_factor + self.reso_offset;
        let pfreq = freq * self.radians_per_sample;
        let b = pfreq / resonance;
        let r = 1.0 - b * 0.5;
        let r2 = 2.0 * r;
        let r22 = r * r;
        let cost = (r2 * f64::cos(pfreq)) / (1.0 + r22);
        let next_b1 = r2 * cost;
        let next_b2 = -r22;
        let next_a0 = (1.0 - r22) * 0.5;

        self.a0 = next_a0;
        self.b1 = next_b1;
        self.b2 = next_b2;
    }
}

