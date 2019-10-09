use super::SampleGenerator;

use rand::prelude::*;

pub struct MultiOscillator {
    sample_rate: f32,
    last_update: u64, // Time of last sample generation

    pub sine_ratio: f32,
    pub tri_ratio: f32,
    pub saw_ratio: f32,
    pub square_ratio: f32,
    pub noise_ratio: f32,

    pub num_voices: u32,
    pub voice_spread: f32,

    state: [State; 7], // State for up to 7 oscillators running in sync
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
    pub fn new(sample_rate: u32) -> MultiOscillator {
        let sample_rate = sample_rate as f32;
        let last_update = 0;
        let sine_ratio = 1.0;
        let tri_ratio = 0.0;
        let saw_ratio = 0.0;
        let square_ratio = 0.0;
        let noise_ratio = 0.0;
        let num_voices = 1;
        let voice_spread = 0.0;
        let freq_offset = 0.0;
        let last_pos = 0.0;
        let last_stabilization = 0;
        let phasor = num::complex::Complex::new(1.0, 0.0);
        let omega = num::complex::Complex::new(0.0, 0.0);
        let stabilizer = num::complex::Complex::new(0.0, 0.0);
        let state = [State{freq_offset, last_pos, last_stabilization, phasor, omega, stabilizer}; 7];
        let osc = MultiOscillator{sample_rate,
                                  last_update,
                                  sine_ratio,
                                  tri_ratio,
                                  saw_ratio,
                                  num_voices,
                                  voice_spread,
                                  square_ratio,
                                  noise_ratio,
                                  state
                                  };
        osc
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
        self.num_voices = 5;
        self.state[0].freq_offset = 0.0;
        self.state[1].freq_offset = -0.6;
        self.state[2].freq_offset = 0.6;
        self.state[3].freq_offset = -1.2;
        self.state[4].freq_offset = 1.2;
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

    fn get_sample_triangle(state: &State, frequency: f32, dt: f32) -> f32 {
        if state.last_pos < 0.25 {
            state.last_pos / 0.25
        } else if state.last_pos < 0.5 {
            1.0 + (1.0 - state.last_pos / 0.25)
        } else if state.last_pos < 0.75 {
            (2.0 - state.last_pos / 0.25)
        } else {
            -1.0 - (3.0 - state.last_pos / 0.25)
        }
    }

    fn get_sample_saw(state: &State, frequency: f32, dt: f32) -> f32 {
        1.0 - (state.last_pos * 2.0)
    }

    fn get_sample_square(state: &State, frequency: f32, dt: f32) -> f32 {
        if state.last_pos < 0.5 {
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
    fn get_sample(&mut self, frequency: f32, sample_clock: u64) -> f32 {
        let dt = sample_clock - self.last_update;
        let dt_f = dt as f32;
        let mut result = 0.0;

        for i in 0..self.num_voices {
            let state: &mut State = &mut self.state[i as usize];
            let freq_offset = (frequency / 100.0) * state.freq_offset;
            let frequency = frequency + freq_offset;
            let freq_speed = frequency / self.sample_rate;
            let diff = freq_speed * dt_f;
            state.last_pos += diff;
            if state.last_pos > 1.0 {
                state.last_pos -= 1.0;
            }

            if self.sine_ratio > 0.0 {
                result += MultiOscillator::get_sample_sine(state, frequency, dt, self.sample_rate) * self.sine_ratio;

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
            if self.tri_ratio > 0.0 {
                result += MultiOscillator::get_sample_triangle(state, frequency, dt_f) * self.tri_ratio;
            }
            if self.saw_ratio > 0.0 {
                result += MultiOscillator::get_sample_saw(state, frequency, dt_f) * self.saw_ratio;
            }
            if self.square_ratio > 0.0 {
                result += MultiOscillator::get_sample_square(state, frequency, dt_f) * self.square_ratio;
            }
            if self.noise_ratio > 0.0 {
                result += MultiOscillator::get_sample_noise(state, frequency, dt_f) * self.noise_ratio;
            }

        }
        self.last_update += dt;

        result / self.num_voices as f32
    }
}

