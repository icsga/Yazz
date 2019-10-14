use std::sync::Arc;
use super::sound::SoundData;

pub struct Envelope {
    sample_rate: f32,
    id: usize,
    rate_mul: f32,
    state: EnvelopeState,
}

#[derive(Default)]
pub struct EnvelopeData {
    pub attack: f32,
    pub decay: f32,
    pub sustain: f32,
    pub release: f32,
}

impl EnvelopeData {
    pub fn init(&mut self) {
        self.attack = 50.0;
        self.decay = 100.0;
        self.sustain = 1.0;
        self.release = 10.0;
    }
}

struct EnvelopeState {
    trigger_time: i64,
    release_time: i64,
    release_level: f32,
    last_update: i64,
    last_value: f32,
    is_held: bool,
    is_running: bool
}

impl Envelope {
    pub fn new(sample_rate: f32, id: usize) -> Envelope {
        Envelope{sample_rate: sample_rate,
                 id: id,
                 rate_mul: sample_rate / 1000.0, // 1 ms
                 state: EnvelopeState{trigger_time: 0,
                                      release_time: 0,
                                      release_level: 0.0,
                                      last_update:0,
                                      last_value: 0.0,
                                      is_held: false,
                                      is_running: false},
        }
    }

    pub fn trigger(&mut self, sample_time: i64) {
        self.state.trigger_time = sample_time;
        self.state.is_held = true;
        self.state.is_running = true;
    }

    pub fn release(&mut self, sample_time: i64) {
        self.state.release_time = sample_time;
        self.state.release_level = self.state.last_value;
        self.state.is_held = false;
    }

    pub fn get_sample(&mut self, sample_time: i64, data: &SoundData) -> f32 {
        let data = data.get_env_data(self.id);
        let attack = data.attack * self.rate_mul;
        let decay = data.decay * self.rate_mul;
        let release = data.release * self.rate_mul;
        if sample_time != self.state.last_update && self.state.is_running {
            let mut dt = (sample_time - self.state.trigger_time) as f32;
            loop {
                if self.state.is_held {
                    // Attack phase
                    if dt < attack {
                        self.state.last_value = dt / attack;
                        break;
                    }
                    // Decay phase
                    dt -= attack;
                    if dt < decay {
                        let sustain_diff = 1.0 - data.sustain;
                        self.state.last_value = (sustain_diff - ((dt / decay) * sustain_diff)) + data.sustain;
                        break;
                    }
                    // Sustain phase
                    self.state.last_value = data.sustain;
                    break;
                }
                // Release phase
                dt = (sample_time - self.state.release_time) as f32;
                if dt < release {
                    // TODO: The case where the envelope is released before the sustain phase is
                    // not correctly handled. We should calculate the adjusted release time
                    // depending on the ratio of release level to sustain level.
                    self.state.last_value = self.state.release_level - ((dt / release) * self.state.release_level);
                    break;
                }
                // Envelope has finished
                self.state.is_running = false;
                self.state.last_value = 0.0;
                break;
            }
        }
        if self.state.last_value > 1.0 {
            panic!("\r\nEnvelope: Got value {}", self.state.last_value);
        }
        self.state.last_value.powi(4)
    }

    pub fn is_running(&self) -> bool {
        self.state.is_running
    }
}
