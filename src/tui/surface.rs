// Implements the control surface of the synth
use std::collections::HashMap;
use std::cell::RefCell;
use std::rc::Rc;

use log::{info, trace, warn};
use termion::{color, cursor};

use super::{Parameter, ParamId, ParameterValue, SynthParam, SoundData};
use super::{Bar, Container, ContainerRef, Controller, Dial, Index, Label, ObserverRef, Scheme, Slider, Value, Widget};

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

        let osc = Rc::new(RefCell::new(Container::new(94, 15)));
        this.add_multi_osc(&mut osc.borrow_mut(), 1, 0, 0);
        this.add_multi_osc(&mut osc.borrow_mut(), 2, 31, 0);
        this.add_multi_osc(&mut osc.borrow_mut(), 3, 63, 0);
        this.window.add_child(osc, 1, 1);

        let env = Rc::new(RefCell::new(Container::new(94, 13)));
        this.add_env(&mut env.borrow_mut(), 1, 0, 0);
        this.add_env(&mut env.borrow_mut(), 2, 20, 0);
        this.window.add_child(env, 1, 16);

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

    fn new_mod_dial_float(&mut self, label: &'static str, min: f64, max: f64, value: f64, log: bool, key: &ParamId) -> ContainerRef {
        let mut c = Container::new(19, 3);
        let label = Label::new(label.to_string(), 10);
        let dial = Dial::new(Value::Float(min), Value::Float(max), Value::Float(value));
        dial.borrow_mut().set_logarithmic(log);
        let modul = Bar::new(Value::Float(0.0), Value::Float(100.0), Value::Float(0.0));
        self.controller.add_observer(key, dial.clone());
        self.mod_targets.insert(*key, modul.clone());
        c.add_child(label, 0, 1);
        c.add_child(dial, 9, 1);
        c.add_child(modul, 0, 2);
        Rc::new(RefCell::new(c))
    }

    fn new_mod_dial_int(&mut self, label: &'static str, min: i64, max: i64, value: i64, log: bool, key: &ParamId) -> ContainerRef {
        let mut c = Container::new(13, 3);
        let label = Label::new(label.to_string(), 8);
        let dial = Dial::new(Value::Int(min), Value::Int(max), Value::Int(value));
        dial.borrow_mut().set_logarithmic(log);
        let modul = Bar::new(Value::Float(0.0), Value::Float(100.0), Value::Float(0.0));
        self.controller.add_observer(key, dial.clone());
        self.mod_targets.insert(*key, modul.clone());
        c.add_child(label, 0, 1);
        c.add_child(dial, 9, 1);
        c.add_child(modul, 0, 2);
        Rc::new(RefCell::new(c))
    }

    fn new_mod_slider_float(&mut self, label: &'static str, min: f64, max: f64, value: f64, log: bool, key: &ParamId) -> ContainerRef {
        // TODO: Add vertical modulation indicator
        let mut c = Container::new(10, 5);
        let label = Label::new(label.to_string(), 8);
        let slider = Slider::new(Value::Float(min), Value::Float(max), Value::Float(value));
        slider.borrow_mut().set_logarithmic(log);
        //let modul = Slider::new(Value::Float(0.0), Value::Float(100.0), Value::Float(0.0));
        self.controller.add_observer(key, slider.clone());
        //self.mod_targets.insert(*key, modul.clone());
        c.add_child(label, 0, 1);
        c.add_child(slider, 0, 2);
        //c.add_child(modul, 1, 2);
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
        let len = title.len();
        let title = Label::new(title, len as Index);
        target.add_child(title, 10 + x_offset, y_offset);

        let mut key = ParamId{function: Parameter::Oscillator, function_id: func_id, parameter: Parameter::Waveform};
        let osc_wave = self.new_mod_dial_int("Waveform", 0, 4, 0, false, &key);
        target.add_child(osc_wave, x_offset, 1 + y_offset);

        Surface::set_key(&mut key, Parameter::Oscillator, func_id, Parameter::Level);
        let osc_level = self.new_mod_dial_float("Level", 0.0, 100.0, 0.0, false, &key);
        target.add_child(osc_level, x_offset, 4 + y_offset);

        Surface::set_key(&mut key, Parameter::Oscillator, func_id, Parameter::Frequency);
        let osc_freq = self.new_mod_dial_int("Pitch", -24, 24, 0, false, &key);
        target.add_child(osc_freq, x_offset, 7 + y_offset);

        Surface::set_key(&mut key, Parameter::Oscillator, func_id, Parameter::Blend);
        let osc_blend = self.new_mod_dial_float("Blend", 0.0, 5.0, 0.0, false, &key);
        target.add_child(osc_blend, x_offset, 10 + y_offset);

        Surface::set_key(&mut key, Parameter::Oscillator, func_id, Parameter::Phase);
        let osc_phase = self.new_mod_dial_float("Phase", 0.0, 1.0, 0.0, false, &key);
        target.add_child(osc_phase, 14 + x_offset, 1 + y_offset);

        Surface::set_key(&mut key, Parameter::Oscillator, func_id, Parameter::Voices);
        let osc_voices = self.new_mod_dial_int("Voices", 1, 7, 1, false, &key);
        target.add_child(osc_voices, 14 + x_offset, 4 + y_offset);

        Surface::set_key(&mut key, Parameter::Oscillator, func_id, Parameter::Spread);
        let osc_spread = self.new_mod_dial_float("Spread", 0.0, 2.0, 0.0, false, &key);
        target.add_child(osc_spread, 14 + x_offset, 7 + y_offset);
    }

    fn add_env(&mut self, target: &mut Container, func_id: usize, x_offset: Index, y_offset: Index) {
        let mut title = "Envelope ".to_string();
        title.push(((func_id as u8) + '0' as u8) as char);
        let len = title.len();
        let title = Label::new(title, len as Index);
        target.add_child(title, x_offset, y_offset);

        let mut key = ParamId{function: Parameter::Envelope, function_id: func_id, parameter: Parameter::Attack};
        let env_attack = self.new_mod_slider_float("A", 0.0, 4000.0, 0.0, true, &key);
        target.add_child(env_attack, x_offset, 1 + y_offset);

        Surface::set_key(&mut key, Parameter::Envelope, func_id, Parameter::Decay);
        let env_decay = self.new_mod_slider_float("D", 0.0, 4000.0, 0.0, true, &key);
        target.add_child(env_decay, 4 + x_offset, 1 + y_offset);

        Surface::set_key(&mut key, Parameter::Envelope, func_id, Parameter::Sustain);
        let env_sustain = self.new_mod_slider_float("S", 0.0, 100.0, 0.0, false, &key);
        target.add_child(env_sustain, 8 + x_offset, 1 + y_offset);

        Surface::set_key(&mut key, Parameter::Envelope, func_id, Parameter::Release);
        let env_release = self.new_mod_slider_float("R", 0.0, 8000.0, 0.0, true, &key);
        target.add_child(env_release, 12 + x_offset, 1 + y_offset);
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
            let param = SynthParam::new(key.function, key.function_id, key.parameter, ParameterValue::NoValue);
            let value = sound.get_value(&param);
            let value = Surface::param_to_widget_value(&value);
            item.borrow_mut().update(value);
        }
    }
}

