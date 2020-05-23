use crate::Float;
//use super::FilterData;
use super::FilterType;

// One pole filter used to construct Oberheim Moog ladder filter
pub struct VAOnePole {
    //sample_rate: Float, // Only needed if filter is used stand-alone
    filter_type: FilterType,
    alpha: Float,
    beta: Float,
    gamma: Float,
    delta: Float,
    epsilon: Float,
    a0: Float,
    feedback: Float,
    z1: Float,
}

impl VAOnePole {
    pub fn new(_sample_rate: Float, filter_type: FilterType) -> Self {
        VAOnePole{
            //sample_rate: sample_rate,
            filter_type: filter_type,
            alpha: 1.0,
            beta: 0.0,
            gamma: 1.0,
            delta: 0.0,
            epsilon: 0.0,
            a0: 1.0,
            feedback: 0.0,
            z1: 0.0}
    }

    pub fn reset(&mut self) {
        self.alpha = 1.0;
        self.beta = 0.0;
        self.gamma = 1.0;
        self.delta = 0.0;
        self.epsilon = 0.0;
        self.a0 = 1.0;
        self.feedback = 0.0;
        self.z1 = 0.0;
    }

    /*
    // Only needed if filter is used stand-alone
    pub fn update(&mut self, _data: &FilterData, freq: Float) {
        let wd = (std::f64::consts::PI * 2.0) * freq;
        let t = 1.0 / self.sample_rate;
        let wa = (2.0 / t) * (wd * t / 2.0).tan();
        let g = wa * t / 2.0;
        self.alpha = g / (1.0 + g);
    }
    */

    pub fn process(&mut self, s: Float) -> Float {
        let s = s * self.gamma + self.feedback + self.epsilon * self.get_feedback_output();
        let vn = (self.a0 * s - self.z1) * self.alpha;
        let out = vn + self.z1;
        self.z1 = vn + out;
        match self.filter_type {
            FilterType::LPF1 => out,
            FilterType::HPF1 => s - out,
            _ => panic!(),
        }
    }

    //pub fn set_feedback(&mut self, fb: Float) { self.feedback = fb; }
    pub fn set_alpha(&mut self, a: Float) { self.alpha = a; }
    pub fn set_beta(&mut self, b: Float) { self.beta = b; }
    pub fn get_feedback_output(&self) -> Float { self.beta * (self.z1 + self.feedback * self.delta) }
}

