use std::cell::RefCell;

pub struct Envelope {
    sample_rate: f32,
    rate_mul: f32,
    attack: f32,
    decay: f32,
    sustain: f32,
    release: f32,
    state: RefCell<EnvelopeState>,
}

struct EnvelopeState {
    trigger_time: u64,
    release_time: u64,
    last_update: u64,
    last_value: f32,
    is_held: bool,
    is_running: bool
}

impl Envelope {
    pub fn new(sample_rate: f32) -> Envelope {
        Envelope{sample_rate: sample_rate,
                 rate_mul: sample_rate / 1000.0, // 1 ms
                 attack: 2000.0,
                 decay: 4000.0,
                 sustain: 0.2,
                 release: 20000.0,
                 state: RefCell::new(EnvelopeState{trigger_time: 0,
                                                   release_time: 0,
                                                   last_update:0,
                                                   last_value: 0.0,
                                                   is_held: false,
                                                   is_running: false})
        }
    }

    pub fn trigger(&self, sample_time: u64) {
        let mut state = self.state.borrow_mut();
        state.trigger_time = sample_time;
        state.is_held = true;
        state.is_running = true;
    }

    pub fn release(&self, sample_time: u64) {
        let mut state = self.state.borrow_mut();
        state.release_time = sample_time;
        state.is_held = false;
    }

    pub fn get_sample(&self, sample_time: u64) -> f32 {
        let mut state = self.state.borrow_mut();
        if sample_time != state.last_update && state.is_running {
            let mut dt = (sample_time - state.trigger_time) as f32;
            loop {
                if dt < self.attack {
                    state.last_value = dt / self.attack;
                    break;
                }
                dt -= self.attack;
                if dt < self.decay {
                    let sustain_diff = 1.0 - self.sustain;
                    state.last_value = (sustain_diff - ((dt / self.decay) * sustain_diff)) + self.sustain;
                    break;
                }
                if state.is_held {
                    state.last_value = self.sustain;
                    break;
                }
                dt = (sample_time - state.release_time) as f32;
                if dt < self.release {
                    state.last_value = self.sustain - ((dt / self.release) * self.sustain);
                    break;
                }
                // Envelope has finished
                state.is_running = false;
                state.last_value = 0.0;
                break;
            }
        }
        state.last_value
    }

    /** Attack: 0 - 1 second in ms */
    pub fn set_attack(&mut self, value: f32) {
        self.attack = self.rate_mul * value;
    }

    pub fn set_decay(&mut self, value: f32) {
        self.decay = self.rate_mul * value;
    }

    pub fn set_sustain(&mut self, value: f32) {
        self.sustain = value;
    }

    pub fn set_release(&mut self, value: f32) {
        self.release = self.rate_mul * value;
    }
}
