// Implements the control surface of the synth
use std::collections::HashMap;
use std::cell::RefCell;
use std::rc::Rc;

use log::{info, trace, warn};
use termion::{color, cursor};

use super::{Parameter, ParamId, ParameterValue, SynthParam, SoundData};
use super::{Bar, Container, ContainerRef, Controller, Dial, Index, Label, ObserverRef, Scheme, Value, Widget};

pub struct Surface {
    window: Container,
    controller: Controller<ParamId>,
    mod_targets: HashMap<ParamId, ObserverRef>, // Maps the modulation indicator to the corresponding parameter key
    colors: Rc<Scheme>,
}

impl Surface {
    pub fn new() -> Surface {
        let window = Container::new(100, 60);
        let controller = Controller::new();
        let mod_targets: HashMap<ParamId, ObserverRef> = HashMap::new();
        let colors = Rc::new(Scheme::new());
        let mut this = Surface{window, controller, mod_targets, colors};

        let osc = Rc::new(RefCell::new(Container::new(94, 40)));
        this.add_multi_osc(&mut osc.borrow_mut(), 1, 0, 0);
        this.add_multi_osc(&mut osc.borrow_mut(), 2, 31, 0);
        this.add_multi_osc(&mut osc.borrow_mut(), 3, 63, 0);
        this.window.add_child(osc, 1, 1);

        this.window.set_position(1, 1);
        this.window.set_color_scheme(this.colors.clone());
        this
    }

    pub fn set_position(&mut self, x: Index, y: Index) {
        self.window.set_position(x, y);
    }

    pub fn draw(&self) {
        self.window.draw();
        print!("{}{}", color::Bg(self.colors.bg_light), color::Fg(self.colors.fg_dark));
    }

    pub fn update_value(&mut self, key: &ParamId, value: Value) {
        self.controller.update(key, value);
    }

    fn new_mod_dial_float(&mut self, label: &'static str, min: f64, max: f64, value: f64, key: &ParamId) -> ContainerRef {
        let mut c = Container::new(19, 3);
        let label = Label::new(label.to_string(), 10);
        let dial = Dial::new(Value::Float(min), Value::Float(max), Value::Float(value));
        let modul = Bar::new(Value::Float(0.0), Value::Float(100.0), Value::Float(0.0));
        self.controller.add_observer(key, dial.clone());
        self.mod_targets.insert(*key, modul.clone());
        c.add_child(label, 1, 1);
        c.add_child(dial, 12, 1);
        c.add_child(modul, 1, 2);
        Rc::new(RefCell::new(c))
    }

    fn new_mod_dial_int(&mut self, label: &'static str, min: i64, max: i64, value: i64, key: &ParamId) -> ContainerRef {
        let mut c = Container::new(13, 3);
        let label = Label::new(label.to_string(), 10);
        let dial = Dial::new(Value::Int(min), Value::Int(max), Value::Int(value));
        let modul = Bar::new(Value::Float(0.0), Value::Float(100.0), Value::Float(0.0));
        self.controller.add_observer(key, dial.clone());
        self.mod_targets.insert(*key, modul.clone());
        c.add_child(label, 1, 1);
        c.add_child(dial, 12, 1);
        c.add_child(modul, 1, 2);
        Rc::new(RefCell::new(c))
    }

    fn set_key(key: &mut ParamId, func: Parameter, func_id: usize, param: Parameter) {
        key.function = func;
        key.function_id = func_id;
        key.parameter = param;
    }

    fn add_multi_osc(&mut self, target: &mut Container, func_id: usize, x_offset: Index, y_offset: Index) {
        let mut title = "Oscillator ".to_string();
        title.push(((func_id as u8) + '0' as u8) as char);
        info!("Title: {}", title);
        let len = title.len();
        let title = Label::new(title, len as Index);
        target.add_child(title, 10 + x_offset, 1 + y_offset);

        let mut key = ParamId{function: Parameter::Oscillator, function_id: func_id, parameter: Parameter::Waveform};
        let osc1_wave = self.new_mod_dial_int("Waveform", 0, 5, 0, &key);
        target.add_child(osc1_wave, 1 + x_offset, 2 + y_offset);

        Surface::set_key(&mut key, Parameter::Oscillator, func_id, Parameter::Level);
        let osc1_level = self.new_mod_dial_float("Level", 0.0, 100.0, 0.0, &key);
        target.add_child(osc1_level, 1 + x_offset, 5 + y_offset);

        Surface::set_key(&mut key, Parameter::Oscillator, func_id, Parameter::Frequency);
        let osc1_freq = self.new_mod_dial_int("Frequency", -24, 24, 0, &key);
        target.add_child(osc1_freq, 1 + x_offset, 8 + y_offset);

        Surface::set_key(&mut key, Parameter::Oscillator, func_id, Parameter::Blend);
        let osc1_blend = self.new_mod_dial_float("Blend", 0.0, 5.0, 0.0, &key);
        target.add_child(osc1_blend, 1 + x_offset, 11 + y_offset);

        Surface::set_key(&mut key, Parameter::Oscillator, func_id, Parameter::Phase);
        let osc1_phase = self.new_mod_dial_float("Phase", 0.0, 1.0, 0.0, &key);
        target.add_child(osc1_phase, 15 + x_offset, 2 + y_offset);

        Surface::set_key(&mut key, Parameter::Oscillator, func_id, Parameter::Voices);
        let osc1_voices = self.new_mod_dial_int("Voices", 1, 7, 1, &key);
        target.add_child(osc1_voices, 15 + x_offset, 5 + y_offset);

        Surface::set_key(&mut key, Parameter::Oscillator, func_id, Parameter::Spread);
        let osc1_spread = self.new_mod_dial_float("Spread", 0.0, 2.0, 0.0, &key);
        target.add_child(osc1_spread, 15 + x_offset, 8 + y_offset);
    }

    fn param_to_widget_value(value: &ParameterValue) -> Value {
        match value {
            ParameterValue::Int(v) => Value::Int(*v),
            ParameterValue::Float(v) => Value::Float((*v).into()),
            ParameterValue::Choice(v) => Value::Int(*v as i64),
            _ => panic!(),
        }
    }

    pub fn update_all(&mut self, sound: &SoundData) {
        for (key, item) in self.controller.observers.iter_mut() {
            info!("Updating item {:?}", key);
            let param = SynthParam::new(key.function, key.function_id, key.parameter, ParameterValue::NoValue);
            let value = sound.get_value(&param);
            let value = Surface::param_to_widget_value(&value);
            item.borrow_mut().update(value);
        }
    }
}

