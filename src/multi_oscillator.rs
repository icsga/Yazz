use super::Float;
use super::SampleGenerator;
use super::sound::SoundData;

use rand::prelude::*;
use serde::{Serialize, Deserialize};
use std::sync::Arc;

use log::{info, trace, warn};

const MAX_VOICES: usize = 7;

pub struct MultiOscillator {
    sample_rate: Float,
    id: usize,
    last_update: i64, // Time of last sample generation
    state: [State; MAX_VOICES], // State for up to 7 oscillators running in sync
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug, Default)]
pub struct MultiOscData {
    pub level: Float,
    pub phase: Float,
    pub sine_ratio: Float,
    pub tri_ratio: Float,
    pub saw_ratio: Float,
    pub square_ratio: Float,
    pub noise_ratio: Float,
    pub num_voices: i64,
    pub voice_spread: Float,
    pub tune_halfsteps: i64,
    pub freq_offset: Float, // Value derived from tune_halfsteps
    pub sync: i64,
    pub key_follow: i64,
}

impl MultiOscData {
    pub fn init(&mut self) {
        self.level = 0.5;
        self.phase = 0.5;
        self.select_wave(0);
        self.set_voice_num(1);
        self.set_freq_offset(0);
        self.sync = 0;
        self.key_follow = 1;
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

    pub fn set_ratios(&mut self, sine_ratio: Float, tri_ratio: Float, saw_ratio: Float, square_ratio: Float, noise_ratio: Float) {
        self.sine_ratio = sine_ratio;
        self.tri_ratio = tri_ratio;
        self.saw_ratio = saw_ratio;
        self.square_ratio = square_ratio;
        self.noise_ratio = noise_ratio;
    }

    pub fn set_ratio(&mut self, ratio: Float) {
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

    pub fn set_voice_num(&mut self, voices: i64) {
        self.num_voices = if voices > MAX_VOICES as i64 { MAX_VOICES as i64 } else { voices };
    }

    pub fn set_voice_spread(&mut self, spread: Float) {
        self.voice_spread = spread;
    }

    pub fn set_freq_offset(&mut self, halfsteps: i64) {
        self.tune_halfsteps = halfsteps;
        let inc: Float = 1.059463;
        self.freq_offset = inc.powf(halfsteps as Float);
    }

    pub fn get_waveform(&self) -> i64 {
        if self.sine_ratio > 0.0 {
            0
        } else if self.tri_ratio > 0.0 {
            1
        } else if self.saw_ratio > 0.0 {
            2
        } else if self.square_ratio > 0.0 {
            3
        } else if self.noise_ratio > 0.0 {
            4
        } else {
            0
        }
    }
}

#[derive(Copy, Clone)]
struct State {
    last_pos: Float,
    freq_shift: Float, // Percentage this voice is shifted from center frequency
    level_shift: Float, // Decrease in level compared to main voice

    // Sinewave
    last_stabilization: i64, // Time of last stabilization
    phasor: num::complex::Complex<Float>, // Phasor with current state
    omega: num::complex::Complex<Float>,
    stabilizer: num::complex::Complex<Float>
}

impl MultiOscillator {
    pub fn new(sample_rate: u32, id: usize) -> MultiOscillator {
        let sample_rate = sample_rate as Float;
        let last_update = 0;
        let last_pos = 0.0;
        let freq_shift = 0.0;
        let level_shift = 1.0;
        let last_stabilization = 0;
        let phasor = num::complex::Complex::new(1.0, 0.0);
        let omega = num::complex::Complex::new(0.0, 0.0);
        let stabilizer = num::complex::Complex::new(0.0, 0.0);
        let state = [State{last_pos, freq_shift, level_shift, last_stabilization, phasor, omega, stabilizer}; 7];
        let osc = MultiOscillator{sample_rate,
                                  id,
                                  last_update,
                                  state
                                  };
        osc
    }

    // Based on http://dsp.stackexchange.com/a/1087
    fn get_sample_sine(state: &mut State, frequency: Float, dt: i64, sample_rate: Float) -> Float {
        // Compute the angular frequency omega in radians
        state.omega.im = 2.0 * 3.141592 * frequency / sample_rate as Float;

        // compute the complex angular coeficient
        let coefficient = state.omega.exp();

        for _ in 0..dt {
                state.phasor *= coefficient;
        }

        state.last_stabilization += dt;
        state.phasor.im // return the 'sine' component of the phasor
    }

    fn get_sample_triangle(state: &State, frequency: Float, phase: Float, dt: Float) -> Float {
        let rate_q1 = 2.0 / phase;
        let rate_q2 = 2.0 / (1.0 - phase);
        let mut pos = state.last_pos + (phase / 2.0);
        if pos > 1.0 { pos -= 1.0 }
        if pos < phase {
            (pos * rate_q1) - 1.0
        } else {
            1.0 - ((pos - phase) * rate_q2)
        }
    }

    fn get_sample_saw(state: &State, frequency: Float, dt: Float) -> Float {
        1.0 - (state.last_pos * 2.0)
    }

    fn get_sample_square(state: &State, frequency: Float, phase: Float, dt: Float) -> Float {
        if state.last_pos < phase {
            1.0
        } else {
            -1.0
        }
    }

    fn get_sample_noise(state: &State, frequency: Float, dt: Float) -> Float {
        (rand::random::<Float>() * 2.0) - 1.0
    }
}

impl SampleGenerator for MultiOscillator {
    fn get_sample(&mut self, frequency: Float, sample_clock: i64, data: &SoundData, reset: bool) -> (Float, bool) {
        let data = data.get_osc_data(self.id);
        let dt = sample_clock - self.last_update;
        let dt_f = dt as Float;
        let mut result = 0.0;
        let mut complete = false;
        if reset {
            self.reset(sample_clock - 1);
        }

        for i in 0..data.num_voices {
            let state: &mut State = &mut self.state[i as usize];
            let freq_diff = (frequency / 100.0) * (data.voice_spread * i as Float) * (1 - (i & 0x01 * 2)) as Float;
            let frequency = frequency + freq_diff;
            let freq_speed = frequency / self.sample_rate;
            let diff = freq_speed * dt_f;
            let mut voice_result = 0.0;
            state.last_pos += diff;
            if state.last_pos > 1.0 {
                // Completed one wave cycle
                state.last_pos -= 1.0;
                complete = true;
            }

            //if data.sine_ratio > 0.0 {
                voice_result += MultiOscillator::get_sample_sine(state, frequency, dt, self.sample_rate) * data.sine_ratio;

                // Periodically stabilize the phasor's amplitude.
                // TODO: Move stabilization into main loop
                if state.last_stabilization > 500 {
                        let a = state.phasor.re;
                        let b = state.phasor.im;
                        state.stabilizer.re = (3.0 - a.powi(2) - b.powi(2)) / 2.0;
                        state.phasor = state.phasor * state.stabilizer;
                        state.last_stabilization = 0;
                }
            //}
            //if data.tri_ratio > 0.0 {
                voice_result += MultiOscillator::get_sample_triangle(state, frequency, data.phase, dt_f) * data.tri_ratio;
            //}
            //if data.saw_ratio > 0.0 {
                voice_result += MultiOscillator::get_sample_saw(state, frequency, dt_f) * data.saw_ratio;
            //}
            //if data.square_ratio > 0.0 {
                voice_result += MultiOscillator::get_sample_square(state, frequency, data.phase, dt_f) * data.square_ratio;
            //}
            //if data.noise_ratio > 0.0 {
                voice_result += MultiOscillator::get_sample_noise(state, frequency, dt_f) * data.noise_ratio;
            //}

            //voice_result *= 1.0 - (i as Float * 0.1);
            result += voice_result;
        }
        self.last_update += dt;
        //result /= data.num_voices as Float;
        result *= data.level;
        if result > 1.0 {
            result = 1.0;
        }
        (result, complete)
    }

    fn reset(&mut self, sample_clock: i64) {
        for state in self.state.iter_mut() {
            state.last_pos = 0.0;
            state.phasor.re = 1.0;
            state.phasor.im = 0.0;
        }
        self.last_update = sample_clock;
    }
}

