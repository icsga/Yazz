use super::MappingType;
use log::info;

#[derive(Debug)]
pub struct MidiLearn {
    pub complete: bool,
    pub ctrl: u64,
    pub mapping_type: MappingType,

    val1: u64,
    val2: u64,
    num_events_received: u64
}

impl MidiLearn {
    pub fn new() -> Self {
        MidiLearn{
            complete: false,
            ctrl: 0,
            mapping_type: MappingType::None,
            val1: 0,
            val2: 0,
            num_events_received: 0
        }
    }

    pub fn reset(&mut self) {
        self.complete = false;
        self.ctrl = 0;
        self.mapping_type = MappingType::None;
        self.val1 = 0;
        self.val2 = 0;
        self.num_events_received = 0;
    }

    pub fn handle_controller(&mut self, controller: u64, value: u64) -> bool {
        info!("handle_controller: ctrl {} val {}, {} events received",
            controller, value, self.num_events_received);
        match self.num_events_received {
            0 => {
                self.ctrl = controller;
                self.val1 = value;
                self.num_events_received = 1;
            }
            1 => {
                if controller == self.ctrl {
                    let diff = (self.val1 as i64 - value as i64).abs();
                    if diff >= 5 {
                        self.val2 = value;
                        self.num_events_received = 2;
                        self.complete = true;
                        self.mapping_type = if diff.abs() >= 125 {
                            MappingType::Relative
                        } else {
                            MappingType::Absolute
                        };
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

    pub fn clear_controller(&mut self) {
        self.reset();
        self.complete = true;
    }
}

// ----------------------------------------------
//                  Unit tests
// ----------------------------------------------

#[test]
fn full_absolute_value_can_be_set() {
    let mut ml = MidiLearn::new();
    assert_eq!(ml.complete, false);

    ml.handle_controller(7, 10);
    assert_eq!(ml.complete, false);

    ml.handle_controller(7, 1);
    assert_eq!(ml.complete, true);
    assert_eq!(ml.ctrl, 7);
    assert_eq!(ml.mapping_type, MappingType::Absolute);
}

#[test]
fn full_relative_value_can_be_set() {
    let mut ml = MidiLearn::new();
    assert_eq!(ml.complete, false);

    ml.handle_controller(7, 127);
    assert_eq!(ml.complete, false);

    ml.handle_controller(7, 0);
    assert_eq!(ml.complete, true);
    assert_eq!(ml.ctrl, 7);
    assert_eq!(ml.mapping_type, MappingType::Relative);
}

#[test]
fn same_values_are_not_counted() {
    let mut ml = MidiLearn::new();
    ml.handle_controller(7, 5);
    assert_eq!(ml.complete, false);
    ml.handle_controller(7, 5);
    assert_eq!(ml.complete, false);
}

#[test]
fn reset_works_after_full_value() {
    let mut ml = MidiLearn::new();
    ml.handle_controller(7, 10);
    ml.handle_controller(7, 1);
    assert_eq!(ml.complete, true);
    ml.reset();
    assert_eq!(ml.complete, false);
}

#[test]
fn reset_works_after_partial_value() {
    let mut ml = MidiLearn::new();
    ml.handle_controller(7, 5);
    ml.reset();
    ml.handle_controller(7, 1);
    assert_eq!(ml.complete, false);
}
