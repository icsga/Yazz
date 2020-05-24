use super::Float;

//use log::info;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Copy, Clone, Default, Debug)]
pub struct EnvelopeData {
    pub attack: Float,
    pub decay: Float,
    pub sustain: Float,
    pub release: Float,
    pub factor: Float,
}

impl EnvelopeData {
    pub fn init(&mut self) {
        self.attack = 15.0;
        self.decay = 15.0;
        self.sustain = 1.0;
        self.release = 15.0;
        self.factor = 1.0;
    }
}

#[derive(Debug)]
enum State {
    Idle,
    Attack,
    Decay,
    Sustain,
    Release,
}

impl State {
    fn next(&self) -> State {
        match self {
            State::Idle => State::Attack,
            State::Attack => State::Decay,
            State::Decay => State::Sustain,
            State::Sustain => State::Release,
            State::Release => State::Idle,
        }
    }
}

#[derive(Debug)]
pub struct Envelope {
    sample_rate: Float,
    rate_mul: Float,

    end_time: i64,
    increment: Float,
    last_update: i64,
    last_value: Float,
    is_held: bool,
    state: State,
}

impl Envelope {
    pub fn new(sample_rate: Float) -> Envelope {
        Envelope{sample_rate: sample_rate,
                 rate_mul: sample_rate / 1000.0, // 1 ms
                 increment: 0.0,
                 end_time: 0,
                 last_update: 0,
                 last_value: 0.0,
                 is_held: false,
                 state: State::Idle,
        }
    }

    pub fn reset(&mut self) {
        self.end_time = 0;
        self.is_held = false;
        self.state = State::Idle;
    }

    pub fn trigger(&mut self, sample_time: i64, data: &EnvelopeData) {
        self.change_state(State::Attack, sample_time, data);
    }

    pub fn release(&mut self, sample_time: i64, data: &EnvelopeData) {
        match self.state {
            State::Release => (), // Don't change to release twice
            _ => self.change_state(State::Release, sample_time, data),
        }
    }

    pub fn get_sample(&mut self, sample_time: i64, data: &EnvelopeData) -> Float {
        if sample_time == self.last_update {
            return self.last_value;
        }
        match self.state {
            State::Idle => return 0.0,
            State::Attack | State::Decay | State::Release => {
                if sample_time >= self.end_time {
                    self.change_state(self.state.next(), sample_time, data);
                } else {
                    self.last_value += self.increment;
                }
            },
            State::Sustain => self.last_value = data.sustain, // Might be updated while not is held,
        }
        if self.last_value > 1.0 {
            self.last_value = 1.0;
        }
        self.last_update = sample_time;
        self.last_value.powf(data.factor)
    }

    pub fn is_running(&self) -> bool {
        match self.state {
            State::Idle => false,
            _ => true
        }
    }
    fn change_state(&mut self, new_state: State, sample_time: i64, data: &EnvelopeData) {
        match new_state {
            State::Idle => self.last_value = 0.0,
            State::Attack => {
                self.end_time = self.calc_end_time(sample_time, data.attack);
                self.increment = self.calc_increment(1.0, data.attack);
                self.is_held = true;
            },
            State::Decay => {
                self.end_time = self.calc_end_time(sample_time, data.decay);
                self.increment = self.calc_increment(data.sustain, data.decay);
            },
            State::Sustain => {
            },
            State::Release => {
                self.end_time = self.calc_end_time(sample_time, data.release);
                self.increment = self.calc_increment(0.0, data.release);
                self.is_held = false;
            },
        }
        //info!("Change to {:?}, last_value = {}, inc = {}, end = {}", new_state, self.last_value, self.increment, self.end_time);
        self.state = new_state;
    }

    fn calc_end_time(&self, sample_time: i64, end_time: Float) -> i64 {
        sample_time + (end_time * self.rate_mul) as i64
    }

    fn calc_increment(&self, target: Float, duration: Float) -> Float {
        (target - self.last_value) / (duration * self.rate_mul)
    }
}
