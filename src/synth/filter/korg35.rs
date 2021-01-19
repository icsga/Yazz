//! Based on the Sallen-Key filter used in the Korg35, as presented in the book
//! "Designing Software Synthesizer Plug-Ins in C++" by Will Pirkle.

use crate::Float;
use super::{FilterData, FilterType, VAOnePole};

/// Sallen-Key filter as used in the Korg35
pub struct K35 {
    sample_rate: Float,
    filter_type: FilterType,

    lpf1: VAOnePole,
    lpf2: VAOnePole,
    hpf1: VAOnePole,
    hpf2: VAOnePole,
    k: Float,
    alpha0: Float,
}

impl K35 {
    pub fn new(sample_rate: Float, filter_type: FilterType) -> Self {
        K35{sample_rate,
            filter_type,
            lpf1: VAOnePole::new(sample_rate, FilterType::LPF1),
            lpf2: VAOnePole::new(sample_rate, FilterType::LPF1),
            hpf1: VAOnePole::new(sample_rate, FilterType::HPF1),
            hpf2: VAOnePole::new(sample_rate, FilterType::HPF1),
            k: 0.01,
            alpha0: 0.0,
        }
    }

    pub fn reset(&mut self) {
        self.lpf1.reset();
        self.lpf2.reset();
        self.hpf1.reset();
        self.hpf2.reset();
    }

    pub fn update(&mut self, data: &FilterData, freq: Float) {
        // Map resonance from [0.0, 1.0] to the range [0.01, 2.0]
        self.k = (2.0 - 0.01) * data.resonance + 0.01;

        let wd = (std::f64::consts::PI * 2.0) * freq;
        let t = 1.0 / self.sample_rate;
        let wa = (2.0 / t) * (wd * t / 2.0).tan();
        let g = wa * t / 2.0;
        let gg = g / (1.0 + g);
        self.lpf1.set_alpha(gg);
        self.lpf2.set_alpha(gg);
        self.hpf1.set_alpha(gg);
        self.hpf2.set_alpha(gg);
        self.alpha0 = 1.0 / (1.0 - self.k * gg + self.k * gg * gg);
        match self.filter_type {
            FilterType::LPF2 => {
                self.lpf2.set_beta((self.k - self.k * gg) / (1.0 + g));
                self.hpf1.set_beta(-1.0 / (1.0 + g));
            }
            FilterType::HPF2 => {
                self.hpf2.set_beta(-1.0 * gg / (1.0 + g));
                self.lpf1.set_beta(1.0 / (1.0 + g));
            }
            _ => panic!(),
        }
    }

    pub fn process(&mut self, s: Float, data: &FilterData) -> Float {
        let mut y: Float;

        match self.filter_type {
            FilterType::LPF2 => {
                let y1 = self.lpf1.process(s);
                let s35 = self.hpf1.get_feedback_output() +
                          self.lpf2.get_feedback_output();
                let mut u = self.alpha0 * (y1 + s35);
                if data.gain > 0.0 {
                    u = (data.gain * u).tanh();
                }
                y = self.k * self.lpf2.process(u);
                self.hpf1.process(y);
            }
            FilterType::HPF2 => {
                let y1 = self.hpf1.process(s);
                let s35 = self.hpf2.get_feedback_output() +
                          self.lpf1.get_feedback_output();
                let u = self.alpha0 * y1 + s35;
                y = self.k * u;
                if data.gain > 0.0 {
                    y = (data.gain * y).tanh();
                }
                self.lpf1.process(self.hpf2.process(y));
            }
            _ => panic!(),
        }
        if self.k > 0.0 {
            y *= 1.0 / self.k;
        }
        y
    }
}

