use log::{info, trace, warn};

#[derive(Debug)]
pub struct MidiLearn {
    pub complete: bool,
    pub ctrl: u64,

    val1: u64,
    val2: u64,
    num_events_received: u64
}

impl MidiLearn {
    pub fn new() -> Self {
        MidiLearn{
            complete: false,
            ctrl: 0,
            val1: 0,
            val2: 0,
            num_events_received: 0
        }
    }

    pub fn reset(&mut self) {
        self.complete = false;
        self.ctrl = 0;
        self.val1 = 0;
        self.val2 = 0;
        self.num_events_received = 0;
    }

    pub fn handle_controller(&mut self, controller: u64, value: u64) -> bool {
        info!("handle_controller: ctrl {} val {}, {} events received", controller, value, self.num_events_received);
        match self.num_events_received {
            0 => {
                self.ctrl = controller;
                self.val1 = value;
                self.num_events_received = 1;
            }
            1 => {
                if controller == self.ctrl {
                    if value != self.val1 {
                        self.val2 = value;
                        self.num_events_received = 2;
                        self.complete = true;
                        info!("handle_controller: MIDI learn complete");
                    }
                } else {
                    self.ctrl = controller;
                    self.val1 = value;
                    self.num_events_received = 1;
                }
            }
            _ => panic!()
        }
        self.complete
    }
}

// ----------------------------------------------
//                  Unit tests
// ----------------------------------------------

#[test]
fn test_full_value_can_be_set() {
    let mut ml = MidiLearn::new();
    assert_eq!(ml.complete, false);

    ml.handle_controller(7, 5);
    assert_eq!(ml.complete, false);

    ml.handle_controller(7, 1);
    assert_eq!(ml.complete, true);
    assert_eq!(ml.ctrl, 7);
}

#[test]
fn test_same_values_are_not_counted() {
    let mut ml = MidiLearn::new();
    ml.handle_controller(7, 5);
    assert_eq!(ml.complete, false);
    ml.handle_controller(7, 5);
    assert_eq!(ml.complete, false);
}

#[test]
fn test_reset_works_after_full_value() {
    let mut ml = MidiLearn::new();
    ml.handle_controller(7, 5);
    ml.handle_controller(7, 1);
    assert_eq!(ml.complete, true);
    ml.reset();
    assert_eq!(ml.complete, false);
}

#[test]
fn test_reset_works_after_partial_value() {
    let mut ml = MidiLearn::new();
    ml.handle_controller(7, 5);
    ml.reset();
    ml.handle_controller(7, 1);
    assert_eq!(ml.complete, false);
}
