use super::SampleGenerator;
use super::synth::SoundData;

use std::sync::Arc;
use rand::prelude::*;

pub struct MultiOscillator {
    sample_rate: f32,
    id: usize,
    last_update: u64, // Time of last sample generation
    state: [State; 7], // State for up to 7 oscillators running in sync
}

#[derive(Default)]
pub struct MultiOscData {
    pub level: f32,
    pub phase: f32,
    pub sine_ratio: f32,
    pub tri_ratio: f32,
    pub saw_ratio: f32,
    pub square_ratio: f32,
    pub noise_ratio: f32,
    pub num_voices: u32,
    pub voice_spread: f32,
    pub freq_offset: f32,
}

impl MultiOscData {
    pub fn init(&mut self) {
        self.select_wave(0);
        self.set_voice_num(1);
        self.level = 1.0;
        self.phase = 0.5;
    }

    pub fn select_wave(&mut self, value: usize) {
        match value {
            0 => self.set_ratios(1.0, 0.0, 0.0, 0.0, 0.0),
            1 => self.set_ratios(0.0, 1.0, 0.0, 0.0, 0.0),
            2 => self.set_ratios(0.0, 0.0, 1.0, 0.0, 0.0),
            3 => self.set_ratios(0.0, 0.0, 0.0, 1.0, 0.0),
            4 => self.set_ratios(0.0, 0.0, 0.0, 0.0, 1.0),
            _ => {}
        }
    }

    pub fn set_ratios(&mut self, sine_ratio: f32, tri_ratio: f32, saw_ratio: f32, square_ratio: f32, noise_ratio: f32) {
        self.sine_ratio = sine_ratio;
        self.tri_ratio = tri_ratio;
        self.saw_ratio = saw_ratio;
        self.square_ratio = square_ratio;
        self.noise_ratio = noise_ratio;
    }

    pub fn set_ratio(&mut self, ratio: f32) {
        if ratio <= 1.0 {
            self.set_ratios(1.0 - ratio, ratio, 0.0, 0.0, 0.0);
        } else if ratio <= 2.0 {
            self.set_ratios(0.0, 1.0 - (ratio - 1.0), ratio - 1.0, 0.0, 0.0);
        } else if ratio <= 3.0 {
            self.set_ratios(0.0, 0.0, 1.0 - (ratio - 2.0), ratio - 2.0, 0.0);
        } else if ratio <= 4.0 {
            self.set_ratios(0.0, 0.0, 0.0, 1.0 - (ratio - 3.0), ratio - 3.0);
        }
    }

    pub fn set_voice_num(&mut self, voices: u32) {
        self.num_voices = 1;
    }
}

#[derive(Copy, Clone)]
struct State {
    freq_offset: f32,
    last_pos: f32,

    // Sinewave
    last_stabilization: u64, // Time of last stabilization
    phasor: num::complex::Complex<f32>, // Phasor with current state
    omega: num::complex::Complex<f32>,
    stabilizer: num::complex::Complex<f32>
}

impl MultiOscillator {
    pub fn new(sample_rate: u32, id: usize) -> MultiOscillator {
        let sample_rate = sample_rate as f32;
        let last_update = 0;
        let freq_offset = 0.0;
        let last_pos = 0.0;
        let last_stabilization = 0;
        let phasor = num::complex::Complex::new(1.0, 0.0);
        let omega = num::complex::Complex::new(0.0, 0.0);
        let stabilizer = num::complex::Complex::new(0.0, 0.0);
        let state = [State{freq_offset, last_pos, last_stabilization, phasor, omega, stabilizer}; 7];
        let osc = MultiOscillator{sample_rate,
                                  id,
                                  last_update,
                                  state
                                  };
        osc
    }

    // Based on http://dsp.stackexchange.com/a/1087
    fn get_sample_sine(state: &mut State, frequency: f32, dt: u64, sample_rate: f32) -> f32 {
        // Compute the angular frequency omega in radians
        state.omega.im = 2.0 * 3.141592 * frequency / sample_rate as f32;

        // compute the complex angular coeficient
        let coefficient = state.omega.exp();

        for _ in 0..dt {
                state.phasor *= coefficient;
        }

        state.last_stabilization += dt;
        state.phasor.im // return the 'sine' component of the phasor
    }

    fn get_sample_triangle(state: &State, frequency: f32, phase: f32, dt: f32) -> f32 {
        /*
        //if state.last_pos < 0.25 {
        let q1 = phase / 2.0;
        let q2 = phase;
        let q3 = phase + ((1.0 - phase) / 2.0);
        if state.last_pos < q1 {
            state.last_pos / q1
        } else if state.last_pos < q2 {
            1.0 + (1.0 - state.last_pos / q1)
        } else if state.last_pos < q3  {
            (2.0 - state.last_pos / q1)
        } else {
            -1.0 - (3.0 - state.last_pos / q1)
        }
        */
        let rate_q1 = 2.0 / phase;
        let rate_q2 = 2.0 / (1.0 - phase);
        if state.last_pos < phase {
            (state.last_pos * rate_q1) - 1.0
        } else {
            1.0 - ((state.last_pos - phase) * rate_q2)
        }
    }

    fn get_sample_saw(state: &State, frequency: f32, dt: f32) -> f32 {
        1.0 - (state.last_pos * 2.0)
    }

    fn get_sample_square(state: &State, frequency: f32, phase: f32, dt: f32) -> f32 {
        if state.last_pos < phase {
            1.0
        } else {
            -1.0
        }
    }

    fn get_sample_noise(state: &State, frequency: f32, dt: f32) -> f32 {
        (rand::random::<f32>() * 2.0) - 1.0
    }
}

impl SampleGenerator for MultiOscillator {
    fn get_sample(&mut self, frequency: f32, sample_clock: u64, data: &SoundData) -> f32 {
        let data = data.get_osc_data(self.id);
        let dt = sample_clock - self.last_update;
        let dt_f = dt as f32;
        let mut result = 0.0;

        for i in 0..data.num_voices {
            let state: &mut State = &mut self.state[i as usize];
            let freq_offset = (frequency / 100.0) * state.freq_offset;
            let frequency = frequency + freq_offset;
            let freq_speed = frequency / self.sample_rate;
            let diff = freq_speed * dt_f;
            state.last_pos += diff;
            if state.last_pos > 1.0 {
                state.last_pos -= 1.0;
            }

            if data.sine_ratio > 0.0 {
                result += MultiOscillator::get_sample_sine(state, frequency, dt, self.sample_rate) * data.sine_ratio;

                // Periodically stabilize the phasor's amplitude.
                // TODO: Move stabilization into main loop
                if state.last_stabilization > 500 {
                        let a = state.phasor.re;
                        let b = state.phasor.im;
                        state.stabilizer.re = (3.0 - a.powi(2) - b.powi(2)) / 2.0;
                        state.phasor = state.phasor * state.stabilizer;
                        state.last_stabilization = 0;
                }
            }
            if data.tri_ratio > 0.0 {
                result += MultiOscillator::get_sample_triangle(state, frequency, data.phase, dt_f) * data.tri_ratio;
            }
            if data.saw_ratio > 0.0 {
                result += MultiOscillator::get_sample_saw(state, frequency, dt_f) * data.saw_ratio;
            }
            if data.square_ratio > 0.0 {
                result += MultiOscillator::get_sample_square(state, frequency, data.phase, dt_f) * data.square_ratio;
            }
            if data.noise_ratio > 0.0 {
                result += MultiOscillator::get_sample_noise(state, frequency, dt_f) * data.noise_ratio;
            }

        }
        self.last_update += dt;
        (result / data.num_voices as f32) * data.level
    }
}

