use crate::Float;
use super::korg35::K35;
use super::ober_moog::OberMoog;
use super::sem::SEM;

use serde::{Serialize, Deserialize};

#[derive(Debug)]
pub enum FilterType {
    LPF1, // 1-pole low pass filter
    LPF2, // 2-pole low pass filter
    LPF4, // 4-pole low pass filter
    BPF2, // 2-pole band pass filter
    BPF4, // 4-pole band pass filter
    BSF2, // 2-pole band stop filter
    HPF1, // 1-pole high pass filter
    HPF2, // 2-pole high pass filter
    HPF4, // 4-pole high pass filter
}

#[derive(Serialize, Deserialize, Copy, Clone, Default, Debug)]
pub struct FilterData {
    pub filter_type: usize,
    pub cutoff: Float,
    pub resonance: Float,
    pub gain: Float,
    pub aux: Float, // General purpose control, usage is filter dependent
    pub env_depth: Float, // Depth of Envevlope 2 cutoff modulation
    pub key_follow: i64,
} 

impl FilterData {
    pub fn init(&mut self) {
        self.filter_type = 1;
        self.cutoff = 3000.0;
        self.resonance = 0.0;
        self.gain = 0.0;
        self.aux = 0.0;
        self.env_depth = 0.0;
        self.key_follow = 0;
    }
}

pub struct Filter {
    last_cutoff: Float,
    last_resonance: Float,

    sem_lpf: SEM,
    sem_bpf: SEM,
    sem_hpf: SEM,
    sem_bsf: SEM,
    k35_lpf: K35,
    k35_hpf: K35,
    om_lpf: OberMoog,
    om_bpf: OberMoog,
    om_hpf: OberMoog,
}

impl Filter {
    pub fn new(sample_rate: u32) -> Filter {
        let sample_rate: Float = sample_rate as Float;
        Filter{last_cutoff: 0.0,
               last_resonance: 0.0,
               sem_lpf: SEM::new(sample_rate, FilterType::LPF2),
               sem_bpf: SEM::new(sample_rate, FilterType::BPF2),
               sem_hpf: SEM::new(sample_rate, FilterType::HPF2),
               sem_bsf: SEM::new(sample_rate, FilterType::BSF2),
               k35_lpf: K35::new(sample_rate, FilterType::LPF2),
               k35_hpf: K35::new(sample_rate, FilterType::HPF2),
               om_lpf: OberMoog::new(sample_rate, FilterType::LPF4),
               om_bpf: OberMoog::new(sample_rate, FilterType::BPF4),
               om_hpf: OberMoog::new(sample_rate, FilterType::HPF4),
        }
    }

    pub fn reset(&mut self) {
        self.sem_lpf.reset();
        self.sem_bpf.reset();
        self.sem_hpf.reset();
        self.sem_bsf.reset();
        self.k35_lpf.reset();
        self.k35_hpf.reset();
        self.om_lpf.reset();
        self.om_bpf.reset();
        self.om_hpf.reset();
    }

    pub fn process(&mut self, sample: Float, data: &mut FilterData, freq: Float, fmod: Float) -> Float {

        // Calculate effective cutoff frequency
        let mut cutoff = data.cutoff;
        if data.key_follow == 1 {
            cutoff = freq * (cutoff / 440.0);
            if cutoff > 8000.0 {
                cutoff = 8000.0;
            } else if cutoff < 1.0 {
                cutoff = 1.0;
            }
        }

        // Apply filter envelope
        if data.env_depth > 0.0 {
            cutoff *= fmod * data.env_depth;
        }

        // If a parameter changed, update coefficients
        if cutoff != self.last_cutoff || data.resonance != self.last_resonance {
            self.update(data, cutoff);
        }

        // Run the sample through the filter
        match data.filter_type {
            0 => sample, // Bypass
            1 => self.sem_lpf.process(sample, data),
            2 => self.sem_bpf.process(sample, data),
            3 => self.sem_hpf.process(sample, data),
            4 => self.sem_bsf.process(sample, data),
            5 => self.k35_lpf.process(sample, data),
            6 => self.k35_hpf.process(sample, data),
            7 => self.om_lpf.process(sample, data),
            8 => self.om_bpf.process(sample, data),
            9 => self.om_hpf.process(sample, data),
            _ => panic!(),
        }
    }

    // Called if cutoff or resonance have changed
    pub fn update(&mut self, data: &FilterData, cutoff: Float) {
        match data.filter_type {
            0 => (),
            1 => self.sem_lpf.update(data, cutoff),
            2 => self.sem_bpf.update(data, cutoff),
            3 => self.sem_hpf.update(data, cutoff),
            4 => self.sem_bsf.update(data, cutoff),
            5 => self.k35_lpf.update(data, cutoff),
            6 => self.k35_hpf.update(data, cutoff),
            7 => self.om_lpf.update(data, cutoff),
            8 => self.om_bpf.update(data, cutoff),
            9 => self.om_hpf.update(data, cutoff),
            _ => panic!(),
        }
        self.last_resonance = data.resonance;
        self.last_cutoff = cutoff;
    }

    // ---------
    // Utilities
    // ---------

    /*
     * These functions were used in the old filter models.

    // TODO: Switch to faster version
    pub fn max(a: Float, b: Float) -> Float {
        if a >= b { a } else { b }
    }

    // Pull values close to zero down to zero
    pub fn normalize(value: Float) -> Float {
        if value >= 0.0 {
            if value > 1e-15 as Float && value < 1e15 as Float {
                value
            } else {
                0.0
            }
        } else {
            if value < -1e-15 as Float && value > -1e15 as Float {
                value
            } else {
                0.0
            }
        }
    }
    */
}
