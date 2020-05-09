use crate::Float;
use super::rlpf::Rlpf;
use super::reson_z::ResonZ;
use super::moog_improved::MoogImproved;

use serde::{Serialize, Deserialize};
use log::{info, trace, warn};

use std::f32;
use std::f64;

#[derive(Serialize, Deserialize, Copy, Clone, Default, Debug)]
pub struct FilterData {
    pub filter_type: usize,
    pub cutoff: Float,
    pub resonance: Float,
    pub gain: Float,
    pub key_follow: i64,
} 

impl FilterData {
    pub fn init(&mut self) {
        self.filter_type = 3; // Moog as default
        self.cutoff = 3000.0;
        self.resonance = 0.0;
        self.gain = 1.0;
        self.key_follow = 0;
    }
}

pub struct Filter {
    last_cutoff: Float,
    last_resonance: Float,

    rlpf: Rlpf,
    reson_z: ResonZ,
    moog_improved: MoogImproved
}

impl Filter {
    pub fn new(sample_rate: u32) -> Filter {
        let sample_rate: Float = sample_rate as Float;
        Filter{last_cutoff: 0.0,
               last_resonance: 0.0,
               rlpf: Rlpf::new(sample_rate),
               reson_z: ResonZ::new(sample_rate),
               moog_improved: MoogImproved::new(sample_rate)
        }
    }

    pub fn reset(&mut self) {
        self.rlpf.reset();
        self.reson_z.reset();
        self.moog_improved.reset();
    }

    pub fn process(&mut self, sample: Float, data: &mut FilterData, freq: Float) -> Float {
        let mut cutoff = data.cutoff;
        if data.key_follow == 1 {
            cutoff += freq;
        }
        if cutoff != self.last_cutoff || data.resonance != self.last_resonance {
            self.update(data, cutoff);
        }

        match data.filter_type {
            0 => sample, // Bypass
            1 => self.rlpf.process(sample, data),
            2 => self.reson_z.process(sample, data),
            3 => self.moog_improved.process(sample, data),
            _ => panic!(),
        }
    }

    // Called if cutoff or resonance have changed
    pub fn update(&mut self, data: &FilterData, freq: Float) {
        match data.filter_type {
            0 => (),
            1 => self.rlpf.update(data, freq),
            2 => self.reson_z.update(data, freq),
            3 => self.moog_improved.update(data, freq),
            _ => panic!(),
        }
        self.last_resonance = data.resonance;
        self.last_cutoff = data.cutoff;
    }

    // ---------
    // Utilities
    // ---------

    // TODO: Switch to faster version
    pub fn max(a: Float, b: Float) -> Float {
        if a >= b { a } else { b }
    }

    // Pull values close to zero down to zero
    pub fn normalize(value: Float) -> Float {
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
}
