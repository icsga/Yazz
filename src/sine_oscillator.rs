// http://dsp.stackexchange.com/a/1087
extern crate num;

use super::Oscillator;
use super::SampleGenerator;
use std::cell::RefCell;

pub struct SineOscillator {
    freq: f32,
    sample_rate: u32,
    state: RefCell<CompSineOscState>
}

struct CompSineOscState {
    last_update: u64, // Time of last sample generation
    last_value: f32,
    last_stabilization: u64, // Time of last stabilization
    phasor: num::complex::Complex<f32>, // Phasor with current state
    omega: num::complex::Complex<f32>,
    stabilizer: num::complex::Complex<f32>
}

impl SineOscillator {
    pub fn new(sample_rate: u32) -> SineOscillator {
        let freq = 440.0;
        let last_update = 0;
        let last_value = 0.0;
        let last_stabilization = 0;
        let phasor = num::complex::Complex::new(1.0, 0.0);
        let omega = num::complex::Complex::new(0.0, 0.0);
        let stabilizer = num::complex::Complex::new(0.0, 0.0);
        let state = RefCell::new(CompSineOscState{last_update, last_value, last_stabilization, phasor, omega, stabilizer});
        let osc = SineOscillator{freq, sample_rate, state};
        osc
    }
}

impl Oscillator for SineOscillator {
    fn set_freq(&mut self, freq: f32) {
        self.freq = freq;
    }

    fn get_freq(&self) -> f32 {
        self.freq
    }
}

impl SampleGenerator for SineOscillator {
    fn get_sample(&self, sample_clock: u64) -> f32 {
        let mut state = self.state.borrow_mut();
        if sample_clock != state.last_update {
            let dt = sample_clock - state.last_update;

            // Compute the angular frequency omega in radians
            state.omega.im = 2.0 * 3.141592 * self.freq / self.sample_rate as f32;

            // compute the complex angular coeficient
            let coefficient = state.omega.exp();

            for _ in 0..dt {
                    state.phasor *= coefficient;
            }

            // Periodically stabilize the phasor's amplitude.
            if state.last_stabilization > 500 {
                    let a = state.phasor.re;
                    let b = state.phasor.im;
                    state.stabilizer.re = (3.0 - a.powi(2) - b.powi(2)) / 2.0;
                    state.phasor = state.phasor * state.stabilizer;
                    state.last_stabilization = 0;
            }

            // advance time
            state.last_update += dt;
            state.last_stabilization += dt;
            // return the 'sine' component of the phasor
            state.last_value = state.phasor.im;
        }
        state.last_value
    }
}

