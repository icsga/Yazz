use super::Float;
use super::SyncValue;
use super::filter::OnePole;

use serde::{Serialize, Deserialize};

const BUFF_LEN: usize = 44100;

#[derive(Serialize, Deserialize, Copy, Clone, Default, Debug)]
pub struct DelayData {
    pub level: Float,
    pub feedback: Float,
    pub time: Float,
    pub sync: SyncValue,
    pub tone: Float,
    pub delay_type: usize,
}

impl DelayData {
    pub fn init(&mut self) {
        self.level = 0.0;
        self.feedback = 0.5;
        self.time = 0.5;
        self.sync = SyncValue::Off;
        self.tone = 1600.0;
    }
}

pub struct Delay {
    sample_rate: Float,
    bb_l: [Float; BUFF_LEN], // Buffer with samples
    bb_r: [Float; BUFF_LEN], // Second buffer with samples
    position: Float,       // Current read/ write position
    quant_pos: usize,    // Last position, quantized to usize
    filter_l: OnePole,
    filter_r: OnePole,
}

impl Delay {
    pub fn new(sample_rate: u32) -> Delay {
        let mut filter_l = OnePole::new(sample_rate);
        let mut filter_r = OnePole::new(sample_rate);
        let sample_rate = sample_rate as Float;
        let bb_l = [0.0; BUFF_LEN];
        let bb_r = [0.0; BUFF_LEN];
        let position = 0.1;
        let quant_pos = 0;
        filter_l.update(2000.0); // Initial frequency at 2kHz
        filter_r.update(2000.0); // Initial frequency at 2kHz
        Delay{sample_rate, bb_l, bb_r, position, quant_pos, filter_l, filter_r}
    }

    pub fn reset(&mut self) {
        for sample in self.bb_l.iter_mut() {
            *sample = 0.0;
        }
        for sample in self.bb_r.iter_mut() {
            *sample = 0.0;
        }
        self.position = 0.1;
        self.quant_pos = 0;
    }

    pub fn process(&mut self, sample_l: Float, sample_r: Float, _sample_clock: i64, data: &DelayData) -> (Float, Float) {
        // TODO: Calculate the passed time using sample_clock
        let step = (self.bb_l.len() as Float / data.time) / self.sample_rate; // The amount of samples we step forward, as float
        let step = Delay::addf(step, 0.0);
        self.position = Delay::addf(self.position, step);
        let new_quant_pos = Delay::add(self.position.round() as usize, 0); // Add 0 to get the wrapping protection
        let num_samples = Delay::diff(new_quant_pos, self.quant_pos); // Actual number of samples we will be stepping over

        // Left side
        // ---------
        // Get the average of all samples we're stepping over
        let mut sample_sum_l = 0.0;
        let mut sample_sum_r = 0.0;
        let mut pos = self.quant_pos;
        for _ in 0..num_samples {
            pos = Delay::add(pos, 1);
            sample_sum_l += self.bb_l[pos];
            sample_sum_r += self.bb_r[pos];
        }
        sample_sum_l /= num_samples as Float;
        sample_sum_r /= num_samples as Float;

        let mut filtered_value_l: Float;
        let mut filtered_value_r: Float;
        pos = self.quant_pos;
        if data.delay_type == 1 {
            // PingPong

            // Mix delay signal to input and update memory. This step exchanges
            // the samples between left and right.
            // (steps through all positions that we jumped over when averaging)
            pos = self.quant_pos;
            for _ in 0..num_samples as usize {
                pos = Delay::add(pos, 1);
                filtered_value_l = self.filter_l.process(sample_l + self.bb_l[pos] * data.feedback);
                filtered_value_r = self.filter_r.process(           self.bb_r[pos] * data.feedback);
                self.bb_l[pos] = filtered_value_r;
                self.bb_r[pos] = filtered_value_l;
            }
        } else {
            // Stereo
            // Mix delay signal to input and update memory
            // (steps through all positions that we jumped over when averaging)
            for _ in 0..num_samples as usize {
                pos = Delay::add(pos, 1);
                filtered_value_l = self.filter_l.process(sample_l + self.bb_l[pos] * data.feedback);
                self.bb_l[pos] = filtered_value_l;
                filtered_value_r = self.filter_r.process(sample_r + self.bb_r[pos] * data.feedback);
                self.bb_r[pos] = filtered_value_r;
            }
        }

        let mixed_sample_l = sample_l + sample_sum_l * data.level;
        let mixed_sample_r = sample_r + sample_sum_r * data.level;
        self.quant_pos = new_quant_pos;
        (mixed_sample_l, mixed_sample_r)
    }

    pub fn update(&mut self, data: &DelayData) {
        self.filter_l.update(data.tone);
        self.filter_r.update(data.tone);
    }

    pub fn update_bpm(&mut self, data: &mut DelayData, bpm: Float) {
        if data.sync == SyncValue::Off {
            return;
        }
        let num_sixteenths = match data.sync {
            SyncValue::Whole => 16.0,
            SyncValue::DottedHalf => 12.0,
            SyncValue::Half => 8.0,
            SyncValue::DottedQuarter => 6.0,
            SyncValue::Quarter => 4.0,
            SyncValue::DottedEigth => 3.0,
            SyncValue::Eigth => 2.0,
            SyncValue::Sixteenth => 1.0,
            SyncValue::Off => panic!(),
        };
        let time = num_sixteenths / ((bpm * 4.0) / 60.0);
        data.time = if time < 0.01 {
            0.01
        } else if time > 1.0 {
            1.0
        } else {
            time
        }
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
