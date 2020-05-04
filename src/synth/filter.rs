use super::Float;
use super::Ringbuffer;

use serde::{Serialize, Deserialize};
use log::{info, trace, warn};

use std::f32;
use std::f64;

#[derive(Serialize, Deserialize, Copy, Clone, Default, Debug)]
pub struct FilterData {
    pub filter_type: usize,
    pub cutoff: Float,
    pub resonance: Float,
    pub gain: Float,
} 

impl FilterData {
    pub fn init(&mut self) {
    }
}

pub struct Filter {
    sample_rate: Float,
    double_sample_rate: Float,
    radians_per_sample: Float,
    resonance: Float, // Might have different value from sound data
    last_cutoff: Float,
    last_resonance: Float,

    y1: Float,
    y2: Float,
    a0: Float,
    b1: Float,
    b2: Float,

    // Moog
    v: [Float; 4],
    dv: [Float; 4],
    tv: [Float; 4],
    x: Float,
    g: Float,
    drive: Float
}

impl Filter {
    pub fn new(sample_rate: u32) -> Filter {
        Filter{sample_rate: sample_rate as Float,
               double_sample_rate: (sample_rate * 2) as Float,
               radians_per_sample: (std::f32::consts::PI * 2.0) / sample_rate as Float,
               resonance: 1.0,
               last_cutoff: 0.0,
               last_resonance: 0.0,
               y1: 0.0, y2: 0.0, a0: 0.0, b1: 0.0, b2: 0.0,
               v: [0.0; 4],
               dv: [0.0; 4],
               tv: [0.0; 4],
               x: 0.0,
               g: 0.0,
               drive: 1.0,
        }
    }

    fn normalize(value: Float) -> Float {
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

    pub fn process(&mut self, sample: Float, sample_clock: i64, data: &mut FilterData) -> Float {
        if data.cutoff != self.last_cutoff || data.resonance != self.last_resonance {
            self.update(data);
        }

        match data.filter_type {
            0 => sample, // Bypass, needs to be set manually
            1 => self.process_rlpf(sample, sample_clock, data),
            2 => self.process_reson_z(sample, sample_clock, data),
            3 => self.process_moog_improved(sample, sample_clock, data),
            _ => panic!(),
        }
    }

    // Called if cutoff or resonance have changed
    pub fn update(&mut self, data: &FilterData) {
        match data.filter_type {
            0 => (),
            1 => self.update_rlpf(data),
            2 => self.update_reson_z(data),
            3 => self.update_moog(data),
            _ => panic!(),
        }
    }

    // adapated from SC3's RLPF
    fn process_rlpf(&mut self, sample: Float, sample_clock: i64, data: &FilterData) -> Float {
        let y0 = self.a0 * sample + self.b1 * self.y1 + self.b2 * self.y2;
        let result = y0 + 2.0 * self.y1 + self.y2;
        self.y2 = Filter::normalize(self.y1);
        self.y1 = Filter::normalize(y0);
        result
    }

    // Adapted from SC3's ResonZ
    fn process_reson_z(&mut self, sample: Float, sample_clock: i64, data: &FilterData) -> Float {
        let y0 = sample + self.b1 * self.y1 + self.b2 * self.y2;
        let result = self.a0 * (y0 - self.y2);
        self.y2 = Filter::normalize(self.y1);
        self.y1 = Filter::normalize(y0);
        result
    }


    fn process_moog_improved(&mut self, sample: Float, sample_clock: i64, data: &FilterData) -> Float {
        // Ported from code found at https://github.com/ddiakopoulos/MoogLadders/
        /*
        Copyright 2012 Stefano D'Angelo <zanga.mail@gmail.com>
        Permission to use, copy, modify, and/or distribute this software for any
        purpose with or without fee is hereby granted, provided that the above
        copyright notice and this permission notice appear in all copies.
        THIS SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES
        WITH REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF
        MERCHANTABILITY AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR
        ANY SPECIAL, DIRECT, INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES
        WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS, WHETHER IN AN
        ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION, ARISING OUT OF
        OR IN CONNECTION WITH THE USE OR PERFORMANCE OF THIS SOFTWARE.
        */
        /*
        This model is based on a reference implementation of an algorithm developed by
        Stefano D'Angelo and Vesa Valimaki, presented in a paper published at ICASSP in 2013.
        This improved model is based on a circuit analysis and compared against a reference
        Ngspice simulation. In the paper, it is noted that this particular model is
        more accurate in preserving the self-oscillating nature of the real filter.
        References: "An Improved Virtual Analog Model of the Moog Ladder Filter"
        Original Implementation: D'Angelo, Valimaki
        */

        // Thermal voltage (26 milliwats at room temperature)
        const VT: Float = 0.312;
        let mut dv0: Float;
        let mut dv1: Float;
        let mut dv2: Float;
        let mut dv3: Float;

        dv0 = -self.g * (((self.drive * sample + data.resonance * self.v[3]) / (2.0 * VT)).tanh() + self.tv[0]);
        self.v[0] += (dv0 + self.dv[0]) / self.double_sample_rate;
        self.dv[0] = dv0;
        self.tv[0] = (self.v[0] / (2.0 * VT)).tanh();
        
        dv1 = self.g * (self.tv[0] - self.tv[1]);
        self.v[1] += (dv1 + self.dv[1]) / self.double_sample_rate;
        self.dv[1] = dv1;
        self.tv[1] = (self.v[1] / (2.0 * VT)).tanh();
        
        dv2 = self.g * (self.tv[1] - self.tv[2]);
        self.v[2] += (dv2 + self.dv[2]) / self.double_sample_rate;
        self.dv[2] = dv2;
        self.tv[2] = (self.v[2] / (2.0 * VT)).tanh();
        
        dv3 = self.g * (self.tv[2] - self.tv[3]);
        self.v[3] += (dv3 + self.dv[3]) / self.double_sample_rate;
        self.dv[3] = dv3;
        self.tv[3] = (self.v[3] / (2.0 * VT)).tanh();
        
        self.v[3]
    }

    fn update_moog(&mut self, data: &FilterData) {
        const VT: Float = 0.312;
        self.x = (std::f32::consts::PI * data.cutoff) / self.sample_rate;
        self.g = 4.0 * std::f32::consts::PI * VT * data.cutoff * (1.0 - self.x) / (1.0 + self.x);
    }

    fn max(a: Float, b: Float) -> Float {
        if a >= b { a } else { b }
    }

    fn update_rlpf(&mut self, data: &FilterData) {
        let qres = Filter::max(0.001, 1.0 / data.resonance);
        let pfreq = data.cutoff * self.radians_per_sample;

        let d = f32::tan(pfreq * qres * 0.5);
        let c = (1.0 - d) / (1.0 + d);
        let cosf = f32::cos(pfreq);
        let next_b1 = (1.0 + c) * cosf;
        let next_b2 = -c;
        let next_a0 = (1.0 + c - next_b1) * 0.25;

        self.resonance = 1.0 / qres;
        self.a0 = next_a0;
        self.b1 = next_b1;
        self.b2 = next_b2;
    }

    fn update_reson_z(&mut self, data: &FilterData) {
        let pfreq = data.cutoff * self.radians_per_sample;
        let b = pfreq / data.resonance;
        let r = 1.0 - b * 0.5;
        let r2 = 2.0 * r;
        let r22 = r * r;
        let cost = (r2 * f32::cos(pfreq)) / (1.0 + r22);
        let next_b1 = r2 * cost;
        let next_b2 = -r22;
        let next_a0 = (1.0 - r22) * 0.5;

        self.a0 = next_a0;
        self.b1 = next_b1;
        self.b2 = next_b2;
    }
}
