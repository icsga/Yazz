// Adapated from SC3's RLPF

use super::{Filter, FilterData};
use crate::Float;

pub struct Rlpf {
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

impl Rlpf {
    pub fn new(sample_rate: Float) -> Self {
        Rlpf{sample_rate: sample_rate,
             radians_per_sample: (std::f64::consts::PI * 2.0) / sample_rate,
             reso_factor: 2000.0 - 30.0, // Filter bandwidth (Q)
             reso_offset: 30.0, // Minimum Q to avoid distortion
             y1: 0.0, y2: 0.0, a0: 0.0, b1: 0.0, b2: 0.0}
    }

    pub fn process(&mut self, sample: Float, data: &FilterData) -> Float {
        let y0 = self.a0 * sample + self.b1 * self.y1 + self.b2 * self.y2;
        let result = y0 + 2.0 * self.y1 + self.y2;
        self.y2 = Filter::normalize(self.y1);
        self.y1 = Filter::normalize(y0);
        result
    }

    pub fn update(&mut self, data: &FilterData, freq: Float) {
        let q = (1.0 - data.resonance) * self.reso_factor + self.reso_offset;

        // According to SC3 docs, this should be reciprocal of Q, so
        // bandwidth / cutoff. That seems to reduce the resonance for
        // lower frequencies.
        //let resonance = q / freq;
        let resonance = q / 2000.0;

        let qres = Filter::max(0.001, resonance);
        let pfreq = freq * self.radians_per_sample;

        let d = f64::tan(pfreq * qres * 0.5);
        let c = (1.0 - d) / (1.0 + d);
        let cosf = f64::cos(pfreq);
        let next_b1 = (1.0 + c) * cosf;
        let next_b2 = -c;
        let next_a0 = (1.0 + c - next_b1) * 0.25;

        self.a0 = next_a0;
        self.b1 = next_b1;
        self.b2 = next_b2;
    }
}
