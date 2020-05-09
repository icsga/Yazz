use super::Float;
use super::filter::OnePole;

use log::{info, trace, warn};
use serde::{Serialize, Deserialize};

const BUFF_LEN: usize = 44100;

#[derive(Serialize, Deserialize, Copy, Clone, Default, Debug)]
pub struct DelayData {
    pub level: Float,
    pub feedback: Float,
    pub time: Float,
    pub tone: Float,
}

impl DelayData {
    pub fn init(&mut self) {
        self.level = 0.0;
        self.feedback = 0.5;
        self.time = 0.5;
        self.tone = 1600.0;
    }
}

pub struct Delay {
    sample_rate: Float,
    bb: [Float; BUFF_LEN], // Buffer with samples
    position: Float,       // Current read/ write position
    quant_pos: usize,    // Last position, quantized to usize
    filter: OnePole,
}

impl Delay {
    pub fn new(sample_rate: u32) -> Delay {
        let mut filter = OnePole::new(sample_rate);
        let sample_rate = sample_rate as Float;
        let bb = [0.0; BUFF_LEN];
        let position = 0.1;
        let quant_pos = 0;
        filter.update(2000.0); // Initial frequency at 2kHz
        Delay{sample_rate, bb, position, quant_pos, filter}
    }

    pub fn reset(&mut self) {
        for sample in self.bb.iter_mut() {
            *sample = 0.0;
        }
        self.position = 0.1;
        self.quant_pos = 0;
    }

    pub fn process(&mut self, sample: Float, sample_clock: i64, data: &DelayData) -> Float {
        let step = (self.bb.len() as Float / data.time) / self.sample_rate; // The amount of samples we step forward, as float
        let step = Delay::addf(step, 0.0);
        self.position = Delay::addf(self.position, step);
        let new_quant_pos = Delay::add(self.position.round() as usize, 0); // Add 0 to get the wrapping protection
        let num_samples = Delay::diff(new_quant_pos, self.quant_pos); // Actual number of samples we will be stepping over
        //info!("sample={}, time={}, step={}, position={}, new_quant_pos={}, num_samples={}", sample, data.time, step, self.position, new_quant_pos, num_samples);

        // Get the average of all samples we're stepping over
        let mut sample_sum = 0.0;
        let mut pos = self.quant_pos;
        for i in 0..num_samples {
            pos = Delay::add(pos, 1);
            sample_sum += self.bb[pos];
        }
        sample_sum /= num_samples as Float;

        // Run sample through filter
        //let sample_sum = self.filter.process(sample_sum);

        // Mix delay signal to input and update memory
        // (steps through all positions that we jumped over)
        let mixed_sample = sample + sample_sum * data.level;
        pos = self.quant_pos;
        let mut filtered_value: Float;
        for i in 0..num_samples as usize {
            pos = Delay::add(pos, 1);
            //filtered_value = self.filter.process(self.bb[pos]);
            self.bb[pos] = self.filter.process(sample + self.bb[pos] * data.feedback);
        }
        self.quant_pos = new_quant_pos;
        mixed_sample
    }

    pub fn update(&mut self, data: &DelayData) {
        self.filter.update(data.tone);
    }

    fn add(mut value: usize, add: usize) -> usize {
        value += add as usize;
        while value >= BUFF_LEN {
            value -= BUFF_LEN;
        }
        value
    }

    fn addf(mut value: Float, add: Float) -> Float {
        value += add;
        while value >= BUFF_LEN as Float {
            value -= BUFF_LEN as Float ;
        }
        value
    }

    fn diff(a: usize, b: usize) -> usize {
        if a > b { a - b } else { (a + BUFF_LEN) -  b}
    }
}
