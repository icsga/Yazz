// Ported from C++ code at https://github.com/ddiakopoulos/MoogLadders/
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

use crate::Float;
use super::FilterData;

pub struct MoogImproved {
    sample_rate: Float,
    double_sample_rate: Float,
    resonance: Float, // Scaled from [0.0, 1.0] to [0.0, 4.0]
    v: [Float; 4],
    dv: [Float; 4],
    tv: [Float; 4],
    x: Float,
    g: Float,
}

impl MoogImproved {
    pub fn new(sample_rate: Float) -> Self {
        MoogImproved{
            sample_rate: sample_rate,
            double_sample_rate: sample_rate * 2.0,
            resonance: 1.0,
            v: [0.0; 4],
            dv: [0.0; 4],
            tv: [0.0; 4],
            x: 0.0,
            g: 0.0,
        }
    }

    pub fn reset(&mut self) {
        for v in &mut self.v {
            *v = 0.0;
        }
        for dv in &mut self.dv {
            *dv = 0.0;
        }
        for tv in &mut self.tv {
            *tv = 0.0;
        }
        self.x = 0.0;
        self.g = 0.0;
    }

    pub fn process(&mut self, sample: Float, data: &FilterData) -> Float {

        // Thermal voltage (26 milliwats at room temperature)
        const VT: Float = 0.312;
        let dv0: Float;
        let dv1: Float;
        let dv2: Float;
        let dv3: Float;

        dv0 = -self.g * (((data.gain * sample + self.resonance * self.v[3]) / (2.0 * VT)).tanh() + self.tv[0]);
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

    pub fn update(&mut self, data: &FilterData, freq: Float) {
        const VT: Float = 0.312;
        self.x = (std::f64::consts::PI * freq) / self.sample_rate;
        self.g = 4.0 * std::f64::consts::PI * VT * freq * (1.0 - self.x) / (1.0 + self.x);
        self.resonance = data.resonance * 4.0;
    }
}

