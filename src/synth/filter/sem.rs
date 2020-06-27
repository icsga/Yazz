use crate::Float;
use super::{FilterData, FilterType};

// One pole filter used to construct Oberheim Moog ladder filter
pub struct SEM {
    sample_rate: Float,
    filter_type: FilterType,
    resonance: Float,

    alpha: Float,
    alpha0: Float,
    rho: Float,
    z11: Float,
    z12: Float,
}

impl SEM {
    pub fn new(sample_rate: Float, filter_type: FilterType) -> Self {
        SEM{
            sample_rate,
            filter_type,
            resonance: 0.5,
            alpha: 1.0,
            alpha0: 1.0,
            rho: 1.0,
            z11: 0.0,
            z12: 0.0
        }
    }

    pub fn reset(&mut self) {
        self.resonance = 0.5;
        self.alpha = 1.0;
        self.alpha0 = 1.0;
        self.rho = 1.0;
        self.z11 = 0.0;
        self.z12 = 0.0;
    }

    pub fn update(&mut self, data: &FilterData, freq: Float) {
        // Map resonance from [0.0, 1.0] to the range [0.5, 25]
        self.resonance = (25.0 - 0.5) * data.resonance + 0.5;

        let wd = (std::f64::consts::PI * 2.0) * freq;
        let t = 1.0 / self.sample_rate;
        let wa = (2.0 / t) * (wd * t / 2.0).tan();
        let g = wa * t / 2.0;
        let r = 1.0 / (2.0 * self.resonance);

        self.alpha0 = 1.0 / (1.0 + (2.0 * r * g) + (g * g));
        self.alpha = g;
        self.rho = 2.0 * r + g;
    }

    pub fn process(&mut self, s: Float, data: &FilterData) -> Float {
        let hpf = self.alpha0 * (s - self.rho * self.z11 - self.z12);
        let mut bpf = self.alpha * hpf + self.z11;
        if data.gain > 0.0 {
            bpf = (bpf + data.gain).tanh();
        }
        let lpf = self.alpha * bpf + self.z12;
        let _sem_bsf = data.aux * hpf + (1.0 - data.aux) * lpf; // TODO: Lost something here when translating
        self.z11 = self.alpha * hpf + bpf;
        self.z12 = self.alpha * bpf + lpf;
        match self.filter_type {
            FilterType::LPF2 => lpf,
            FilterType::BPF2 => bpf,
            FilterType::HPF2 => hpf,
            FilterType::BSF2 => {
                let r = 1.0 / (2.0 * self.resonance);
                s - 2.0 * r * bpf
            }
            _ => panic!(),
        }
    }
}


