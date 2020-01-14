use super::Float;
use super::Ringbuffer;

use serde::{Serialize, Deserialize};
use log::{info, trace, warn};

#[derive(Serialize, Deserialize, Copy, Clone, Default, Debug)]
pub struct FilterData {
    pub filter_type: usize,
    pub cutoff: Float,
    pub resonance: Float,
    pub gain: Float,
} 

impl FilterData {
    pub fn init(&mut self) {
    }
}

pub struct Filter {
    sample_rate: Float,
    radians_per_sample: Float,
    resonance: Float, // Might have different value from sound data
    last_cutoff: Float,
    last_resonance: Float,

    y1: Float,
    y2: Float,
    a0: Float,
    b1: Float,
    b2: Float,
}

impl Filter {
    pub fn new(sample_rate: u32) -> Filter {
        Filter{sample_rate: sample_rate as Float,
               radians_per_sample: (std::f32::consts::PI * 2.0) / sample_rate as Float,
               resonance: 1.0,
               last_cutoff: 0.0,
               last_resonance: 0.0,
               y1: 0.0, y2: 0.0, a0: 0.0, b1: 0.0, b2: 0.0}
    }

    fn normalize(value: Float) -> Float {
        if value >= 0.0 {
            if value > 1e-15 as Float && value < 1e15 as Float {
                value
            } else {
                0.0
            }
        } else {
            if value < -1e-15 as Float && value > -1e15 as Float {
                value
            } else {
                0.0
            }
        }
    }

    pub fn process(&mut self, sample: Float, sample_clock: i64, data: &mut FilterData) -> Float {
        if data.cutoff != self.last_cutoff || data.resonance != self.last_resonance {
            self.update(data);
        }

        match data.filter_type {
            0 => self.process_rlpf(sample, sample_clock, data),
            1 => self.process_reson_z(sample, sample_clock, data),
            _ => panic!(),
        }
    }

    // Called if cutoff or resonance have changed
    pub fn update(&mut self, data: &FilterData) {
        match data.filter_type {
            0 => self.update_rlpf(data),
            1 => self.update_reson_z(data),
            _ => panic!(),
        }
    }

    // adapated from SC3's RLPF
    fn process_rlpf(&mut self, sample: Float, sample_clock: i64, data: &FilterData) -> Float {
        let y0 = self.a0 * sample + self.b1 * self.y1 + self.b2 * self.y2;
        let result = y0 + 2.0 * self.y1 + self.y2;
        self.y2 = Filter::normalize(self.y1);
        self.y1 = Filter::normalize(y0);
        result
    }

    // Adapted from SC3's ResonZ
    fn process_reson_z(&mut self, sample: Float, sample_clock: i64, data: &FilterData) -> Float {
        let y0 = sample + self.b1 * self.y1 + self.b2 * self.y2;
        let result = self.a0 * (y0 - self.y2);
        self.y2 = Filter::normalize(self.y1);
        self.y1 = Filter::normalize(y0);
        result
    }

    fn max(a: Float, b: Float) -> Float {
        if a >= b { a } else { b }
    }

    fn update_rlpf(&mut self, data: &FilterData) {
        let qres = Filter::max(0.001, 1.0 / data.resonance);
        let pfreq = data.cutoff * self.radians_per_sample;

        let d = f32::tan(pfreq * qres * 0.5);
        let c = (1.0 - d) / (1.0 + d);
        let cosf = f32::cos(pfreq);
        let next_b1 = (1.0 + c) * cosf;
        let next_b2 = -c;
        let next_a0 = (1.0 + c - next_b1) * 0.25;

        self.resonance = 1.0 / qres;
        self.a0 = next_a0;
        self.b1 = next_b1;
        self.b2 = next_b2;
    }

    fn update_reson_z(&mut self, data: &FilterData) {
        let pfreq = data.cutoff * self.radians_per_sample;
        let b = pfreq / data.resonance;
        let r = 1.0 - b * 0.5;
        let r2 = 2.0 * r;
        let r22 = r * r;
        let cost = (r2 * f32::cos(pfreq)) / (1.0 + r22);
        let next_b1 = r2 * cost;
        let next_b2 = -r22;
        let next_a0 = (1.0 - r22) * 0.5;

        self.a0 = next_a0;
        self.b1 = next_b1;
        self.b2 = next_b2;
    }
}
