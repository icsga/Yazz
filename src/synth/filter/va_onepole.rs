use crate::Float;
use super::{Filter, FilterData};

// One pole filter used to construct Oberheim Moog ladder filter
struct VAOnePole {
    sample_rate: Float,
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
    pub fn new(sample_rate: Float) -> Self {
        VAOnePole{
            sample_rate: sample_rate,
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

    pub fn tick(&mut self, s: Float) -> Float {
        let s = s * self.gamma + self.feedback + self.epsilon * self.get_feedback_output();
        let vn = (self.a0 * s - self.z1) * self.alpha;
        let out = vn + self.z1;
        self.z1 = vn + out;
        out
    }

    pub fn set_feedback(&mut self, fb: Float) { self.feedback = fb; }
    pub fn set_alpha(&mut self, a: Float) { self.alpha = a; }
    pub fn set_beta(&mut self, b: Float) { self.beta = b; }
    pub fn get_feedback_output(&self) -> Float { self.beta * (self.z1 + self.feedback * self.delta) }
}

