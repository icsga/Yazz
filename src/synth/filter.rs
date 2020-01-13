use super::Float;
use super::Ringbuffer;

use serde::{Serialize, Deserialize};
use log::{info, trace, warn};

#[derive(Serialize, Deserialize, Copy, Clone, Debug)]
pub enum FilterType {
    Default
}

impl Default for FilterType {
    fn default() -> Self { FilterType::Default }
}

#[derive(Serialize, Deserialize, Copy, Clone, Default, Debug)]
pub struct FilterData {
    pub filter_type: FilterType,
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

    pub y1: Float,
    pub y2: Float,
    pub a0: Float,
    pub b1: Float,
    pub b2: Float,
}

impl Filter {
    pub fn new(sample_rate: u32) -> Filter {
        Filter{sample_rate: sample_rate as Float,
               radians_per_sample: (std::f32::consts::PI * 2.0) / sample_rate as Float,
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

    // Adapted from SC3's ResonZ
    pub fn process(&mut self, sample: Float, sample_clock: i64, data: &FilterData) -> Float {
        let y0 = sample + self.b1 * self.y1 + self.b2 * self.y2;
        let result = self.a0 * (y0 - self.y2);
        self.y2 = Filter::normalize(self.y1);
        self.y1 = Filter::normalize(y0);
        return result;
    }

    // Called if cutoff or resonance have changed
    pub fn update(&mut self, data: &mut FilterData) {
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
