use super::Float;

//use log::info;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Copy, Clone, Default, Debug)]
pub struct EnvelopeData {
    pub delay: Float,
    pub attack: Float,
    pub decay: Float,
    pub sustain: Float,
    pub release: Float,
    pub factor: Float,
    pub looping: bool,
    pub reset_to_zero: bool,
}

impl EnvelopeData {
    pub fn init(&mut self) {
        self.delay = 0.0;
        self.attack = 15.0;
        self.decay = 15.0;
        self.sustain = 1.0;
        self.release = 15.0;
        self.factor = 1.0;
        self.looping = false;
        self.reset_to_zero = false;
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum EnvState {
    Idle,
    Delay,
    Attack,
    Decay,
    Sustain,
    Release,
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
    state: EnvState,
}

impl Envelope {
    pub fn new(sample_rate: Float) -> Envelope {
        Envelope{sample_rate,
                 rate_mul: sample_rate / 1000.0, // Samples per ms
                 increment: 0.0,
                 end_time: 0,
                 last_update: 0,
                 last_value: 0.0,
                 is_held: false,
                 state: EnvState::Idle,
        }
    }

    pub fn reset(&mut self) {
        self.end_time = 0;
        self.is_held = false;
        self.state = EnvState::Idle;
    }

    pub fn trigger(&mut self, sample_time: i64, data: &EnvelopeData) {
        self.is_held = true;
        self.select_initial_state(sample_time, data);
    }

    pub fn release(&mut self, sample_time: i64, data: &EnvelopeData) {
        self.is_held = false;
        match self.state {
            EnvState::Release => (), // Don't change to release twice
            _ => self.change_state(EnvState::Release, sample_time, data),
        }
    }

    pub fn get_sample(&mut self, sample_time: i64, data: &EnvelopeData) -> Float {
        if sample_time == self.last_update {
            return self.last_value;
        }
        match self.state {
            EnvState::Idle => return 0.0,
            EnvState::Delay => {
                if sample_time >= self.end_time {
                    self.change_state(EnvState::Attack, sample_time, data);
                }
            }
            EnvState::Attack => {
                self.last_value += self.increment;
                if sample_time >= self.end_time {
                    self.change_state(EnvState::Decay, sample_time, data);
                }
            }
            EnvState::Decay => {
                self.last_value += self.increment;
                if sample_time >= self.end_time {
                    if data.looping {
                        self.change_state(EnvState::Release, sample_time, data);
                    } else {
                        self.change_state(EnvState::Sustain, sample_time, data);
                    }
                }
            }
            EnvState::Sustain => self.last_value = data.sustain, // Might be updated while not is held,
            EnvState::Release => {
                self.last_value += self.increment;
                if sample_time >= self.end_time {
                    if self.is_held && data.looping {
                        self.select_initial_state(sample_time, data);
                    } else {
                        self.change_state(EnvState::Idle, sample_time, data);
                    }
                }
            }
        }
        if self.last_value > 1.0 {
            self.last_value = 1.0;
        } else if self.last_value < 0.0 {
            self.last_value = 0.0;
        }
        self.last_update = sample_time;
        self.last_value.powf(data.factor)
    }

    pub fn is_running(&self) -> bool {
        match self.state {
            EnvState::Idle => false,
            _ => true
        }
    }

    fn change_state(&mut self, new_state: EnvState, sample_time: i64, data: &EnvelopeData) {
        match new_state {
            EnvState::Idle => self.last_value = 0.0,
            EnvState::Delay => {
                self.last_value = 0.0;
                self.end_time = self.calc_end_time(sample_time, data.delay);
            }
            EnvState::Attack => {
                if data.reset_to_zero {
                    self.last_value = 0.0;
                }

                // We have a fixed slope based on the attack time. Starting on
                // a non-zero value will shorten the time to reach 1.0.
                self.increment = self.calc_increment(0.0, 1.0, data.attack);

                // If we're not starting at zero, time_frac will tell us how
                // much faster we will reach the target.
                let time_frac = 1.0 - self.last_value;
                self.end_time = self.calc_end_time(sample_time, data.attack * time_frac);
            }
            EnvState::Decay => {
                // Decay always starts after hitting 1.0. Slope is based on the
                // decay time and sustain level.
                self.increment = self.calc_increment(1.0, data.sustain, data.decay);
                self.end_time = self.calc_end_time(sample_time, data.decay);
            }
            EnvState::Sustain => {
            }
            EnvState::Release => {
                // Release phase can be entered from any of the other phases,
                // so the slope depends on the current level. That keeps the
                // actual release time constant (it's a time, not a rate).
                // TODO: Maybe change that, to keep it consistent with attack
                //       and decay?
                self.increment = self.calc_increment(self.last_value, 0.0, data.release);
                self.end_time = self.calc_end_time(sample_time, data.release);
            }
        }

        //info!("Change to {:?} at {}, last_value = {}, inc = {}, end = {}",
            //new_state, sample_time, self.last_value, self.increment, self.end_time);

        self.state = new_state;
    }

    fn select_initial_state(&mut self, sample_time: i64, data: &EnvelopeData) {
        if data.delay > 0.0 {
            self.change_state(EnvState::Delay, sample_time, data);
        } else {
            self.change_state(EnvState::Attack, sample_time, data);
        }
    }

    fn calc_increment(&self, from: Float, to: Float, duration: Float) -> Float {
        (to - from) / (duration * self.rate_mul)
    }

    fn calc_end_time(&self, sample_time: i64, end_time: Float) -> i64 {
        (sample_time as Float + (end_time as Float * self.rate_mul as Float)).round() as i64
    }
}

// ----------------------------------------------
//                  Unit tests
// ----------------------------------------------

#[cfg(test)]
mod tests {

use super::{Envelope, EnvelopeData, EnvState};
use super::super::Float;

struct TestContext {
    pub env: Envelope,
    pub data: EnvelopeData,
    last_time: i64,
}

use log::info;
use flexi_logger::{Logger, opt_format};
static mut LOGGER_INITIALIZED: bool = false;

impl TestContext {

    fn new() -> TestContext {
        // Setup logging if required
        unsafe {
            if LOGGER_INITIALIZED == false {
                Logger::with_env_or_str("myprog=debug, mylib=warn")
                                        .log_to_file()
                                        .directory("log_files")
                                        .format(opt_format)
                                        .start()
                                        .unwrap();
                LOGGER_INITIALIZED = true;
            }
        }
        TestContext{
            env: Envelope::new(1000.0), // Sample rate 1000 Hz for easier calculations
            data: EnvelopeData{
                delay: 10.0,
                attack: 10.0,
                decay: 10.0,
                sustain: 0.5,
                release: 10.0,
                factor: 1.0,
                looping: false,
                reset_to_zero: false,
            },
            last_time: 0,
        }
    }

    fn advance_time(&mut self, time: i64) {
        while self.last_time < time {
            self.env.get_sample(self.last_time, &self.data);
            self.last_time += 1;
        }
    }

    // Some forwarding functions for cleaner tests
    pub fn reset(&mut self) {
        self.env.reset();
    }

    pub fn trigger(&mut self, time: i64) {
        self.env.trigger(time, &self.data);
    }

    pub fn release(&mut self, time: i64) {
        self.advance_time(time);
        self.env.release(time, &self.data);
    }

    pub fn get_sample(&mut self, time: i64) -> Float {
        self.advance_time(time);
        self.env.get_sample(time, &self.data)
    }

    pub fn state(&self) -> EnvState {
        self.env.state
    }

    pub fn is_held(&self) -> bool {
        self.env.is_held
    }
}

fn close(a: Float, b: Float) -> bool {
    let result = (a - b).abs();
    if result > 0.00001 {
        false
    } else {
        true
    }
}

#[test]
fn reset_clears_state() {
    let mut c = TestContext::new();
    c.reset();
    assert_eq!(c.state(), EnvState::Idle);
    assert_eq!(c.is_held(), false);
}

#[test]
fn trigger_starts_envelope() {
    let mut c = TestContext::new();
    c.trigger(0);
    assert_eq!(c.state(), EnvState::Delay);
    assert_eq!(c.is_held(), true);
}

#[test]
fn env_gives_right_values_without_retrigger() {
    let mut c = TestContext::new();
    c.trigger(0);

    // Delay phase
    assert!(close(c.get_sample(0), 0.0));
    assert_eq!(c.state(), EnvState::Delay);

    // Attack phase
    assert!(close(c.get_sample(10), 0.0));
    assert_eq!(c.state(), EnvState::Attack);

    // Decay phase
    assert!(close(c.get_sample(20), 1.0));
    assert_eq!(c.state(), EnvState::Decay);

    // Sustain phase
    assert!(close(c.get_sample(30), 0.5));
    assert_eq!(c.state(), EnvState::Sustain);

    // Still Sustain phase
    assert!(close(c.get_sample(40), 0.5));
    assert_eq!(c.state(), EnvState::Sustain);

    // Release phase
    assert!(close(c.get_sample(50), 0.5));
    c.release(50);
    assert_eq!(c.state(), EnvState::Release);

    // Finished
    assert!(close(c.get_sample(60), 0.0));
    assert_eq!(c.state(), EnvState::Idle);
}

#[test]
fn release_in_delay() {
    let mut c = TestContext::new();
    c.trigger(0);

    // Run until halfpoint of delay phase
    assert!(close(c.get_sample(5), 0.0));
    assert_eq!(c.state(), EnvState::Delay);
    c.release(5);

    // Release phase with matching slope. The envelope should stay open until
    // the release time has passed.
    assert!(close(c.get_sample(5), 0.0));
    assert_eq!(c.state(), EnvState::Release);
    assert!(close(c.get_sample(15), 0.0));
    assert_eq!(c.state(), EnvState::Idle);
    assert_eq!(c.is_held(), false);
}

#[test]
fn release_in_attack() {
    let mut c = TestContext::new();
    c.trigger(0);

    // Run until halfpoint of attack phase
    assert!(close(c.get_sample(15), 0.5));
    assert_eq!(c.state(), EnvState::Attack);
    c.release(15);

    // Release phase with matching slope
    assert!(close(c.get_sample(20), 0.25));
    assert!(close(c.get_sample(25), 0.0));
    assert_eq!(c.state(), EnvState::Idle);
    assert_eq!(c.is_held(), false);
}

#[test]
fn release_in_decay() {
    let mut c = TestContext::new();
    c.trigger(0);

    // Run until halfpoint of decay phase
    assert!(close(c.get_sample(25), 0.75));
    assert_eq!(c.state(), EnvState::Decay);
    c.release(25);

    // Release phase with matching slope
    assert!(close(c.get_sample(30), 0.375));
    assert!(close(c.get_sample(35), 0.0));
    assert_eq!(c.state(), EnvState::Idle);
    assert_eq!(c.is_held(), false);
}

#[test]
fn retrigger_in_delay() {
    let mut c = TestContext::new();
    c.trigger(0);

    assert!(close(c.get_sample(5), 0.0));
    assert_eq!(c.state(), EnvState::Delay);
    c.trigger(5);

    // Delay time should be reset on retrigger
    assert!(close(c.get_sample(15), 0.0));
    assert_eq!(c.state(), EnvState::Attack);
}

#[test]
fn retrigger_in_attack() {
    let mut c = TestContext::new();
    c.data.delay = 0.0;
    c.trigger(0);

    assert!(close(c.get_sample(5), 0.5));
    assert_eq!(c.state(), EnvState::Attack);
    c.trigger(5);

    // We should still hit 1 at the end of the original attack period
    assert!(close(c.get_sample(10), 1.0));
    assert_eq!(c.state(), EnvState::Decay);
}

#[test]
fn retrigger_in_decay() {
    let mut c = TestContext::new();
    c.data.delay = 0.0;
    c.trigger(0);

    assert!(close(c.get_sample(15), 0.75));
    assert_eq!(c.state(), EnvState::Decay);
    c.trigger(15);
    assert_eq!(c.state(), EnvState::Attack);

    assert!(close(c.get_sample(18), 1.0));
    assert_eq!(c.state(), EnvState::Decay);
}

} // mod test
