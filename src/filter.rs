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

    // Coefficients
    // TODO: Not so nice to store these in the sound data, since they are
    // computed automatically. Find a better way to do this.
    pub a0: Float,
    pub a1: Float,
    pub a2: Float,
    pub a3: Float,
    pub a4: Float,
    pub b0: Float,
    pub b1: Float,
    pub b2: Float,
    pub b3: Float,
    pub b4: Float,
} 

impl FilterData {
    pub fn init(&mut self) {
    }
}

pub struct Filter {
    sample_rate: Float,
    buff_in: [Float; 4],
    buff_out: [Float; 4],
    rb_in: Ringbuffer,
    rb_out: Ringbuffer,
}

impl Filter {
    pub fn new(sample_rate: u32) -> Filter {
        let sample_rate = sample_rate as Float;
        let buff_in = [0.0; 4];
        let buff_out = [0.0; 4];
        let rb_in = Ringbuffer::new();
        let rb_out = Ringbuffer::new();
        Filter{sample_rate, buff_in, buff_out, rb_in, rb_out}
    }

    pub fn process(&mut self, sample: Float, sample_clock: i64, data: &FilterData) -> Float {
        sample
    }

    pub fn update(&mut self, data: &mut FilterData) {
        // TODO: Use conditional compilation to get the right constant for f32/ f64
        let y = ((std::f32::consts::PI * data.cutoff) / self.sample_rate).tan();
        data.b0 = y;
        data.b1 = y;
        data.a1 = y - 1.0;
        info!("Filter: y={}, b0={}, b1={}, a1={}", y, data.b0, data.b1, data.a1);
    }
}
