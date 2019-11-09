extern crate num;

use super::Float;
use super::SampleGenerator;
use super::SoundData;

use log::{info, trace, warn};
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Copy, Clone, Debug, Default)]
pub struct LfoData {
    pub frequency: Float,
}

impl LfoData {
    pub fn init(&mut self) {
        self.frequency = 1.0;
    }
}

pub struct Lfo {
    sample_rate: u32,
    last_update: i64, // Time of last sample generation
    last_stabilization: i64, // Time of last stabilization
    phasor: num::complex::Complex<Float>, // Phasor with current state
    omega: num::complex::Complex<Float>,
    stabilizer: num::complex::Complex<Float>
}

impl Lfo {
    pub fn new(sample_rate: u32) -> Lfo {
        let last_update = 0;
        let last_stabilization = 0;
        let phasor = num::complex::Complex::new(1.0, 0.0);
        let omega = num::complex::Complex::new(0.0, 0.0);
        let stabilizer = num::complex::Complex::new(0.0, 0.0);
        let lfo = Lfo{sample_rate, last_update, last_stabilization, phasor, omega, stabilizer};
        lfo
    }

    // Based on http://dsp.stackexchange.com/a/1087
    pub fn get_sample(&mut self, sample_clock: i64, data: &LfoData, reset: bool) -> (Float, bool) {
        let dt = sample_clock - self.last_update;

        // Compute the angular frequency omega in radians
        // TODO: Use conditional compilation to select the correct PI constant for f32/ f64
        self.omega.im = 2.0 * std::f32::consts::PI * data.frequency / self.sample_rate as Float;

        // compute the complex angular coeficient
        let coefficient = self.omega.exp();

        for _ in 0..dt {
                self.phasor *= coefficient;
        }

        // Periodically stabilize the phasor's amplitude.
        if self.last_stabilization > 500 {
                let a = self.phasor.re;
                let b = self.phasor.im;
                self.stabilizer.re = (3.0 - a.powi(2) - b.powi(2)) / 2.0;
                self.phasor = self.phasor * self.stabilizer;
                self.last_stabilization = 0;
        }

        // advance time
        self.last_update += dt;
        self.last_stabilization += dt;
        (self.phasor.im, false) // return the 'sine' component of the phasor
    }

    fn reset(&mut self, sample_clock: i64) {
        self.phasor.re = 1.0;
        self.phasor.im = 0.0;
        self.last_update = sample_clock;
    }
}

