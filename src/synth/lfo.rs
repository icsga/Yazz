extern crate num;

use super::Float;

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
    pub phase: Float,
    pub amount: Float,
}

impl LfoData {
    pub fn init(&mut self) {
        self.select_wave(0);
        self.frequency = 1.0;
        self.phase = 0.0;
        self.amount = 1.0;
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
    position: Float, // Position in the wave at last update, going from 0.0 to 1.0
    last_value: Float, // Value of previous iteration (only used for S&H)
}

impl Lfo {
    pub fn new(sample_rate: u32) -> Lfo {
        let last_update = 0;
        let position = 0.0;
        let last_value = 0.0;
        let lfo = Lfo{sample_rate, last_update, position, last_value};
        lfo
    }

    fn get_sample_sine(&mut self) -> Float {
        (self.position * 2.0 * std::f64::consts::PI).sin()
    }

    fn get_sample_triangle(&mut self) -> Float {
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

    fn get_sample_saw_down(&mut self) -> Float {
        1.0 - (self.position * 2.0)
    }

    fn get_sample_saw_up(&mut self) -> Float {
        (self.position * 2.0) - 1.0
    }

    fn get_sample_square(&mut self, phase: Float) -> Float {
        if self.position < phase {
            1.0
        } else {
            -1.0
        }
    }

    fn get_sample_noise(&mut self) -> Float {
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
        let result: Float;
        let mut complete = false;
        if reset {
            self.reset(sample_clock - 1, data.phase);
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
            LfoWaveform::Sine => self.get_sample_sine(),
            LfoWaveform::Tri => self.get_sample_triangle(),
            LfoWaveform::Saw => self.get_sample_saw_up(),
            LfoWaveform::SawDown => self.get_sample_saw_down(),
            LfoWaveform::Square => self.get_sample_square(0.5),
            LfoWaveform::Noise => self.get_sample_noise(),
            LfoWaveform::SnH => self.get_sample_snh(complete),
        } * data.amount;

        self.last_update += dt;
        if result > 1.0 {
            panic!("LFO overrun");
        }
        (result, complete)
    }

    pub fn reset(&mut self, sample_clock: i64, phase: Float) {
        self.last_update = sample_clock;
        self.position = phase;
    }
}
