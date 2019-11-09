extern crate num;

use super::Float;
use super::SampleGenerator;

pub struct SineOscillator {
    sample_rate: u32,
    last_update: u64, // Time of last sample generation
    last_stabilization: u64, // Time of last stabilization
    phasor: num::complex::Complex<Float>, // Phasor with current state
    omega: num::complex::Complex<Float>,
    stabilizer: num::complex::Complex<Float>
}

impl SineOscillator {
    pub fn new(sample_rate: u32) -> SineOscillator {
        let last_update = 0;
        let last_stabilization = 0;
        let phasor = num::complex::Complex::new(1.0, 0.0);
        let omega = num::complex::Complex::new(0.0, 0.0);
        let stabilizer = num::complex::Complex::new(0.0, 0.0);
        let osc = SineOscillator{sample_rate, last_update, last_stabilization, phasor, omega, stabilizer};
        osc
    }
}

impl SampleGenerator for SineOscillator {
    // Based on http://dsp.stackexchange.com/a/1087
    fn get_sample(&self, frequency: Float, sample_clock: u64) -> Float {
        let dt = sample_clock - self.last_update;

        // Compute the angular frequency omega in radians
        self.omega.im = 2.0 * 3.141592 * frequency / self.sample_rate as Float;

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
        self.phasor.im // return the 'sine' component of the phasor
    }
}

