// http://dsp.stackexchange.com/a/1087
extern crate num;

use super::Oscillator;
use super::SampleGenerator;
use std::cell::RefCell;

pub struct TriangleOscillator {
    freq: f32,
    sample_rate: u32,
    state: RefCell<TriangleOscState>
}

struct TriangleOscState {
    last_update: u64, // Time of last sample generation
    last_value: f32,
    last_angle: f32,
}

impl TriangleOscillator {
    pub fn new(sample_rate: u32) -> TriangleOscillator {
        let freq = 440.0;
        let last_update = 0;
        let last_value = 0.0;
        let last_angle = 0.0;
        let state = RefCell::new(TriangleOscState{last_update, last_value, last_angle});
        let osc = TriangleOscillator{freq, sample_rate, state};
        osc
    }
}

impl Oscillator for TriangleOscillator {
    fn set_freq(&mut self, freq: f32) {
        self.freq = freq;
    }

    fn get_freq(&self) -> f32 {
        self.freq
    }
}

impl SampleGenerator for TriangleOscillator {
    fn get_sample(&self, sample_clock: u64) -> f32 {
        let mut state = self.state.borrow_mut();
        if sample_clock != state.last_update {
            let dt = sample_clock - state.last_update;

            let freq_speed = self.freq / self.sample_rate as f32;
            let diff = freq_speed * dt as f32;

            state.last_angle += diff;
            if state.last_angle > 1.0 {
                state.last_angle -= 1.0;
            }

            state.last_value = if state.last_angle < 0.25 {
                state.last_angle / 0.25
            } else if state.last_angle < 0.5 {
                1.0 + (1.0 - state.last_angle / 0.25)
            } else if state.last_angle < 0.75 {
                (2.0 - state.last_angle / 0.25)
            } else {
                -1.0 - (3.0 - state.last_angle / 0.25)
            };

            // advance time
            state.last_update += dt;
        }
        state.last_value
    }
}

