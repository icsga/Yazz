extern crate num;

use super::Float;
use super::SampleGenerator;
use super::SoundData;

use log::{info, trace, warn};
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub enum LfoWaveform {
    Sine,
    Tri,
    Saw,
    SawDown,
    Square,
    SnH,
    Noise,
}

impl Default for LfoWaveform {
    fn default() -> Self { LfoWaveform::Sine }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default)]
pub struct LfoData {
    pub waveform: LfoWaveform,
    pub frequency: Float,
}

impl LfoData {
    pub fn init(&mut self) {
        self.frequency = 1.0;
    }

    pub fn select_wave(&mut self, value: usize) {
        self.waveform = match value {
            0 => LfoWaveform::Sine,
            1 => LfoWaveform::Tri,
            2 => LfoWaveform::Saw,
            3 => LfoWaveform::SawDown,
            4 => LfoWaveform::Square,
            5 => LfoWaveform::SnH,
            6 => LfoWaveform::Noise,
            _ => panic!(),
        }
    }

    pub fn get_waveform(&self) -> usize {
        match self.waveform {
            LfoWaveform::Sine => 0,
            LfoWaveform::Tri => 1,
            LfoWaveform::Saw => 2,
            LfoWaveform::SawDown => 3,
            LfoWaveform::Square => 4,
            LfoWaveform::SnH => 5,
            LfoWaveform::Noise => 6,
        }
    }
}

pub struct Lfo {
    pub sample_rate: u32,
    last_update: i64, // Time of last sample
    position: Float, // Position in the wave at last update
    last_value: Float, // Value of previous iteration (only used for S&H)

    // Sine-specific values
    stabilization_counter: i64, // Time since last stabilization
    phasor: num::complex::Complex<Float>, // Phasor with current state
    omega: num::complex::Complex<Float>,
    stabilizer: num::complex::Complex<Float>,
}

impl Lfo {
    pub fn new(sample_rate: u32) -> Lfo {
        let last_update = 0;
        let position = 0.0;
        let last_value = 0.0;
        let stabilization_counter = 0;
        let phasor = num::complex::Complex::new(1.0, 0.0);
        let omega = num::complex::Complex::new(0.0, 0.0);
        let stabilizer = num::complex::Complex::new(0.0, 0.0);
        let lfo = Lfo{sample_rate, last_update, position, last_value, stabilization_counter, phasor, omega, stabilizer};
        lfo
    }

    // Based on http://dsp.stackexchange.com/a/1087
    fn get_sample_sine(&mut self, frequency: Float, dt: i64) -> Float {
        // Compute the angular frequency omega in radians
        self.omega.im = 2.0 * 3.141592 * frequency / self.sample_rate as Float;

        // compute the complex angular coeficient
        let coefficient = self.omega.exp();

        for _ in 0..dt {
            self.phasor *= coefficient;
        }

        // Periodically stabilize the phasor's amplitude.
        self.stabilization_counter += dt;
        if self.stabilization_counter > 500 {
                let a = self.phasor.re;
                let b = self.phasor.im;
                self.stabilizer.re = (3.0 - a.powi(2) - b.powi(2)) / 2.0;
                self.phasor = self.phasor * self.stabilizer;
                self.stabilization_counter = 0;
        }

        let mut value = self.phasor.im; // return the 'sine' component of the phasor
        if value > 1.0 {
            value = 1.0;
        } else if value < -1.0 {
            value = -1.0;
        }
        value
    }

    fn get_sample_triangle(&mut self, dt: Float, phase: Float) -> Float {
        /* This version allows for asymmetrical triangle, but we're not using
         * it at the moment.
        let rate_q1 = 2.0 / phase;
        let rate_q2 = 2.0 / (1.0 - phase);
        let mut pos = self.position + (phase / 2.0);
        if pos > 1.0 { pos -= 1.0 }
        if pos < phase {
            (pos * rate_q1) - 1.0
        } else {
            1.0 - ((pos - phase) * rate_q2)
        }
        */
        // Faster version
        1.0 - 2.0 * (2.0 * self.position - 1.0).abs()
    }

    fn get_sample_saw_down(&mut self, dt: Float) -> Float {
        1.0 - (self.position * 2.0)
    }

    fn get_sample_saw_up(&mut self, dt: Float) -> Float {
        (self.position * 2.0) - 1.0
    }

    fn get_sample_square(&mut self, dt: Float, phase: Float) -> Float {
        if self.position < phase {
            1.0
        } else {
            -1.0
        }
    }

    fn get_sample_noise(&mut self, dt: Float) -> Float {
        (rand::random::<Float>() * 2.0) - 1.0
    }

    fn get_sample_snh(&mut self, get_new_value: bool) -> Float {
        if get_new_value {
            self.last_value = (rand::random::<Float>() * 2.0) - 1.0;
        }
        self.last_value
    }

    pub fn get_sample(&mut self, sample_clock: i64, data: &LfoData, reset: bool) -> (Float, bool) {
        let dt = sample_clock - self.last_update;
        let dt_f = dt as Float;
        let mut result: Float;
        let mut complete = false;
        if reset {
            self.reset(sample_clock - 1);
            complete = true;
        }

        let freq_speed = data.frequency / self.sample_rate as Float;
        let diff = freq_speed * dt_f;
        self.position += diff;
        if self.position > 1.0 {
            // Completed one wave cycle
            self.position -= 1.0;
            complete = true;
        }

        result = match data.waveform {
            LfoWaveform::Sine => self.get_sample_sine(data.frequency, dt),
            LfoWaveform::Tri => self.get_sample_triangle(dt_f, 0.5),
            LfoWaveform::Saw => self.get_sample_saw_up(dt_f),
            LfoWaveform::SawDown => self.get_sample_saw_down(dt_f),
            LfoWaveform::Square => self.get_sample_square(dt_f, 0.5),
            LfoWaveform::Noise => self.get_sample_noise(dt_f),
            LfoWaveform::SnH => self.get_sample_snh(complete),
        };

        self.last_update += dt;
        if result > 1.0 {
            panic!("LFO overrun");
        }
        (result, complete)
    }

    pub fn reset(&mut self, sample_clock: i64) {
        self.last_update = sample_clock;
        self.position = 0.0;
        self.phasor.re = 1.0;
        self.phasor.im = 0.0;
        self.stabilization_counter = 0;
    }
}
