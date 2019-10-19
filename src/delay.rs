use serde::{Serialize, Deserialize};

const BUFF_LEN: usize = 44100;

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct DelayData {
    pub level: f32,
    pub feedback: f32,
    pub speed: f32,
}

impl DelayData {
    pub fn init(&mut self) {
        self.level = 0.5;
        self.feedback = 0.5;
        self.speed = 1.0;
    }
}

pub struct Delay {
    sample_rate: f32,
    bb: [f32; BUFF_LEN],
    pos: f32,
    last_quant_pos: usize,
}

impl Delay {
    pub fn new(sample_rate: u32) -> Delay {
        let sample_rate = sample_rate as f32;
        let bb = [0.0; BUFF_LEN];
        let pos = 0.0;
        let last_quant_pos = 0;
        Delay{sample_rate, bb, pos, last_quant_pos}
    }

    pub fn process(&mut self, sample: f32, sample_clock: i64, data: &DelayData) -> f32 {
        let step = (self.bb.len() as f32 / data.speed) / self.sample_rate;
        self.pos = self.addf(self.pos, step);
        let new_pos = self.pos + step;
        let mut quant_pos = new_pos.round() as usize;
        quant_pos = if quant_pos > BUFF_LEN { quant_pos - BUFF_LEN } else { quant_pos };
        let num_samples = if quant_pos > self.last_quant_pos { quant_pos - self.last_quant_pos } else { (quant_pos + BUFF_LEN) - self.last_quant_pos };
        let mut replace_sample: f32;

        let mut old_sample = 0.0;
        let mut pos = self.last_quant_pos;
        for i in 0..num_samples {
            pos = self.add(pos, 1);
            old_sample += self.bb[pos];
        }
        old_sample /= num_samples as f32;

        let mixed_sample = sample + old_sample * data.level;
        //replace_sample = sample + old_sample * data.feedback; // Either use the summed value, or
        //  read the individual values again

        pos = self.last_quant_pos;
        for i in 0..num_samples as usize {
            pos = self.add(pos, 1);
            replace_sample = sample + self.bb[pos] * data.feedback;
            self.bb[pos] = replace_sample;
        }
        self.last_quant_pos = quant_pos;
        mixed_sample
    }

    fn add(&self, mut value: usize, add: usize) -> usize {
        value += add as usize;
        value = if value >= BUFF_LEN { value - BUFF_LEN } else { value };
        value
    }

    fn addf(&self, mut value: f32, add: f32) -> f32 {
        value += add;
        value = if value >= BUFF_LEN as f32 { value - BUFF_LEN as f32 } else { value };
        value
    }
}
