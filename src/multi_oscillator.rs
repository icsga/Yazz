use super::SampleGenerator;

pub struct MultiOscillator {
    sample_rate: u32,
    last_update: u64, // Time of last sample generation

    sine_ratio: f32,
    tri_ratio: f32,
    square_ratio: f32,

    // General status
    last_pos: f32,

    // Sinewave status
    last_stabilization: u64, // Time of last stabilization
    phasor: num::complex::Complex<f32>, // Phasor with current state
    omega: num::complex::Complex<f32>,
    stabilizer: num::complex::Complex<f32>
}

impl MultiOscillator {
    pub fn new(sample_rate: u32) -> MultiOscillator {
        let last_update = 0;
        let last_pos = 0.0;
        let sine_ratio = 1.0;
        let tri_ratio = 0.0;
        let square_ratio = 0.0;
        let last_stabilization = 0;
        let phasor = num::complex::Complex::new(1.0, 0.0);
        let omega = num::complex::Complex::new(0.0, 0.0);
        let stabilizer = num::complex::Complex::new(0.0, 0.0);
        let osc = MultiOscillator{sample_rate,
                                  last_update,
                                  last_pos,
                                  sine_ratio,
                                  tri_ratio,
                                  square_ratio,
                                  last_stabilization,
                                  phasor,
                                  omega,
                                  stabilizer};
        osc
    }

    pub fn set_ratios(&mut self, sine_ratio: f32, tri_ratio: f32, square_ratio: f32) {
        self.sine_ratio = sine_ratio;
        self.tri_ratio = tri_ratio;
        self.square_ratio = square_ratio;
    }

    // Based on http://dsp.stackexchange.com/a/1087
    fn get_sample_sine(&mut self, frequency: f32, sample_clock: u64) -> f32 {
        let dt = sample_clock - self.last_update;

        // Compute the angular frequency omega in radians
        self.omega.im = 2.0 * 3.141592 * frequency / self.sample_rate as f32;

        // compute the complex angular coeficient
        let coefficient = self.omega.exp();

        for _ in 0..dt {
                self.phasor *= coefficient;
        }

        // Periodically stabilize the phasor's amplitude.
        if self.last_stabilization > 500 {
                let a = self.phasor.re;
                let b = self.phasor.im;
                self.stabilizer.re = (3.0 - a.powi(2) - b.powi(2)) / 2.0;
                self.phasor = self.phasor * self.stabilizer;
                self.last_stabilization = 0;
        }

        // advance time
        self.last_update += dt;
        self.last_stabilization += dt;
        self.phasor.im // return the 'sine' component of the phasor
    }

    fn get_sample_triangle(&mut self, frequency: f32, sample_clock: u64) -> f32 {
        let dt = sample_clock - self.last_update;

        let freq_speed = frequency / self.sample_rate as f32;
        let diff = freq_speed * dt as f32;

        self.last_pos += diff;
        if self.last_pos > 1.0 {
            self.last_pos -= 1.0;
        }

        // advance time
        self.last_update += dt;

        if self.last_pos < 0.25 {
            self.last_pos / 0.25
        } else if self.last_pos < 0.5 {
            1.0 + (1.0 - self.last_pos / 0.25)
        } else if self.last_pos < 0.75 {
            (2.0 - self.last_pos / 0.25)
        } else {
            -1.0 - (3.0 - self.last_pos / 0.25)
        }
    }

    fn get_sample_square(&mut self, frequency: f32, sample_clock: u64) -> f32 {
        let dt = sample_clock - self.last_update;
        let freq_speed = frequency / self.sample_rate as f32;
        let diff = freq_speed * dt as f32;

        self.last_pos += diff;
        if self.last_pos > 1.0 {
            self.last_pos -= 1.0;
        }
        self.last_update += dt;
        if self.last_pos < 0.5 {
            1.0
        } else {
            -1.0
        }
    }
}

impl SampleGenerator for MultiOscillator {
    fn get_sample(&mut self, frequency: f32, sample_clock: u64) -> f32 {
        let mut result = 0.0;

        if self.sine_ratio > 0.0 {
            result += self.get_sample_sine(frequency, sample_clock) * self.sine_ratio;
        }
        if self.tri_ratio > 0.0 {
            result += self.get_sample_triangle(frequency, sample_clock) * self.tri_ratio;
        }
        if self.square_ratio > 0.0 {
            result += self.get_sample_square(frequency, sample_clock) * self.square_ratio;
        }
        result
    }
}

