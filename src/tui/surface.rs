// Implements the control surface of the synth
use std::collections::HashMap;
use std::cell::RefCell;
use std::rc::Rc;

use log::{info, trace, warn};

use super::{Parameter, ParamId};
use super::{Bar, Container, ContainerRef, Controller, Dial, Index, Label, ObserverRef, Value, Widget};

pub struct Surface {
    window: Container,
    controller: Controller<ParamId>,
    mod_targets: HashMap<ParamId, ObserverRef>, // Maps the modulation indicator to the corresponding parameter key
}

impl Surface {
    pub fn new() -> Surface {
        let window = Container::new(100, 60);
        let controller = Controller::new();
        let mod_targets = HashMap::new();
        let mut this = Surface{window, controller, mod_targets};
        let mod_targets: HashMap<ParamId, ObserverRef> = HashMap::new();
        let key = ParamId{function: Parameter::Oscillator, function_id: 1, parameter: Parameter::Level};

        let osc1_level = this.new_mod_dial_float("Level", 0.0, 100.0, 0.0, key);
        this.window.add_child(osc1_level, 1, 1);

        this.window.set_position(1, 1);
        this
    }

    pub fn set_position(&mut self, x: Index, y: Index) {
        self.window.set_position(x, y);
    }

    pub fn draw(&self) {
        self.window.draw();
    }

    pub fn update_value(&mut self, key: ParamId, value: Value) {
        self.controller.update(key, value);
    }

    fn new_mod_dial_float(&mut self, label: &'static str, min: f64, max: f64, value: f64, key: ParamId) -> ContainerRef {
        let mut c = Container::new(19, 3);
        let label = Label::new(label);
        let dial = Dial::new(Value::Float(min), Value::Float(max), Value::Float(value));
        let modul = Bar::new(Value::Float(0.0), Value::Float(100.0), Value::Float(0.0));
        self.controller.add_observer(key, dial.clone());
        self.mod_targets.insert(key, modul.clone());
        c.add_child(label, 1, 1);
        c.add_child(dial, 18, 1);
        c.add_child(modul, 1, 2);
        Rc::new(RefCell::new(c))
    }
}

