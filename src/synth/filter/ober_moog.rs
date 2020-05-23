//! Based on the Moog ladder filter with Oberheim variations, as presented in
//! the book "Designing Software Synthesizer Plug-Ins in C++" by Will Pirkle.

use crate::Float;
use super::{FilterData, FilterType};
use super::VAOnePole;

pub struct OberMoog {
    sample_rate: Float,
    filter_type: FilterType,

    lpf1: VAOnePole,
    lpf2: VAOnePole,
    lpf3: VAOnePole,
    lpf4: VAOnePole,

    k: Float,
    alpha0: Float,
    gamma: Float,
    oberheim_coefs: [Float; 5],
}

impl OberMoog {

    pub fn new(sample_rate: Float, filter_type: FilterType) -> Self {
        OberMoog{
            sample_rate: sample_rate,
            filter_type: filter_type,
            lpf1: VAOnePole::new(sample_rate, FilterType::LPF1),
            lpf2: VAOnePole::new(sample_rate, FilterType::LPF1),
            lpf3: VAOnePole::new(sample_rate, FilterType::LPF1),
            lpf4: VAOnePole::new(sample_rate, FilterType::LPF1),
            k: 0.0,
            alpha0: 0.0,
            gamma: 0.0,
            oberheim_coefs: [0.0, 0.0, 0.0, 0.0, 0.0]
        }
    }

    pub fn reset(&mut self) {
        self.lpf1.reset();
        self.lpf2.reset();
        self.lpf3.reset();
        self.lpf4.reset();
    }

    pub fn update(&mut self, data: &FilterData, freq: Float) {
        // Map resonance from [0.0, 1.0] to the range [0.0, 4.0]
        self.k = 4.0 * data.resonance;

        // prewarp for BZT
        let wd = 2.0 * std::f64::consts::PI * freq;
        let t = 1.0 / self.sample_rate;
        let wa = (2.0 / t) * (wd * t / 2.0).tan();
        let g = wa * t / 2.0;

        // Feedforward coeff
        let gg = g / (1.0 + g);

        self.lpf1.set_alpha(gg);
        self.lpf2.set_alpha(gg);
        self.lpf3.set_alpha(gg);
        self.lpf4.set_alpha(gg);

        self.lpf1.set_beta(gg * gg * gg / (1.0 + g));
        self.lpf2.set_beta(gg * gg / (1.0 + g));
        self.lpf3.set_beta(gg / (1.0 + g));
        self.lpf4.set_beta(1.0 / (1.0 + g));

        self.gamma = gg * gg * gg * gg;
        self.alpha0 = 1.0 / (1.0 + self.k * self.gamma);

        // Oberheim variations / LPF4
        match self.filter_type {
            FilterType::LPF4 => {
                self.oberheim_coefs[0] =  0.0;
                self.oberheim_coefs[1] =  0.0;
                self.oberheim_coefs[2] =  0.0;
                self.oberheim_coefs[3] =  0.0;
                self.oberheim_coefs[4] =  1.0;
            }
            FilterType::LPF2 => {
                self.oberheim_coefs[0] =  0.0;
                self.oberheim_coefs[1] =  0.0;
                self.oberheim_coefs[2] =  1.0;
                self.oberheim_coefs[3] =  0.0;
                self.oberheim_coefs[4] =  0.0;
            }
            FilterType::BPF4 => {
                self.oberheim_coefs[0] =  0.0;
                self.oberheim_coefs[1] =  0.0;
                self.oberheim_coefs[2] =  4.0;
                self.oberheim_coefs[3] = -8.0;
                self.oberheim_coefs[4] =  4.0;
            }
            FilterType::BPF2 => {
                self.oberheim_coefs[0] =  0.0;
                self.oberheim_coefs[1] =  2.0;
                self.oberheim_coefs[2] = -2.0;
                self.oberheim_coefs[3] =  0.0;
                self.oberheim_coefs[4] =  0.0;
            }
            FilterType::HPF4 => {
                self.oberheim_coefs[0] =  1.0;
                self.oberheim_coefs[1] = -4.0;
                self.oberheim_coefs[2] =  6.0;
                self.oberheim_coefs[3] = -4.0;
                self.oberheim_coefs[4] =  1.0;
            }
            FilterType::HPF2 => {
                self.oberheim_coefs[0] =  1.0;
                self.oberheim_coefs[1] = -2.0;
                self.oberheim_coefs[2] =  1.0;
                self.oberheim_coefs[3] =  0.0;
                self.oberheim_coefs[4] =  0.0;
            }
            _ => panic!(),
        }
    }

    pub fn process(&mut self, s: Float, data: &FilterData) -> Float {
        let input = s;

        let sigma = self.lpf1.get_feedback_output() +
                    self.lpf2.get_feedback_output() +
                    self.lpf3.get_feedback_output() +
                    self.lpf4.get_feedback_output();

        // For passband gain compensation
        //input *= 1.0 + data.aux * self.k;

        // calculate input to first filter
        let mut u = (input - self.k * sigma) * self.alpha0;

        if data.gain > 0.0 {
            u = (data.gain * u).tanh();
        }

        let lp1 = self.lpf1.process(u);
        let lp2 = self.lpf2.process(lp1);
        let lp3 = self.lpf3.process(lp2);
        let lp4 = self.lpf4.process(lp3);

        // Calculate result (Oberheim variation)
        self.oberheim_coefs[0] * u +
        self.oberheim_coefs[1] * lp1 +
        self.oberheim_coefs[2] * lp2 +
        self.oberheim_coefs[3] * lp3 +
        self.oberheim_coefs[4] * lp4
    }
}
