// Implements the control surface of the synth
use std::collections::HashMap;
use std::cell::RefCell;
use std::hash::Hash;
use std::rc::Rc;

use log::{info, trace, warn};
use termion::{color, cursor};

use super::{Parameter, ParamId, ParameterValue, SynthParam, SoundData, UiMessage};
use super::{Bar, Button, Container, ContainerRef, Controller, Dial, Index,
            Label, MouseHandler, ObserverRef, Scheme, Slider, Value,
            ValueDisplay, Widget};

pub struct Surface {
    window: Container<ParamId>,
    controller: Controller<ParamId>,
    mod_targets: HashMap<ParamId, ObserverRef>, // Maps the modulation indicator to the corresponding parameter key
    mouse_handler: MouseHandler<ParamId>,
    colors: Rc<Scheme>,
}

impl Surface {
    pub fn new() -> Surface {
        let window = Container::new();
        let controller = Controller::new();
        let mod_targets: HashMap<ParamId, ObserverRef> = HashMap::new();
        let mouse_handler = MouseHandler::new();
        let colors = Rc::new(Scheme::new());
        let mut this = Surface{window,
                               controller,
                               mod_targets,
                               mouse_handler,
                               colors};

        let osc: ContainerRef<ParamId> = Rc::new(RefCell::new(Container::new()));
        osc.borrow_mut().enable_border(true);
        this.add_multi_osc(&mut osc.borrow_mut(), 1, 0, 0);
        this.add_multi_osc(&mut osc.borrow_mut(), 2, 31, 0);
        this.add_multi_osc(&mut osc.borrow_mut(), 3, 63, 0);
        this.add_child(osc, 1, 1);

        let env: ContainerRef<ParamId> = Rc::new(RefCell::new(Container::new()));
        env.borrow_mut().enable_border(true);
        this.add_env(&mut env.borrow_mut(), 1, 1, 0);
        let (x, _) = env.borrow().get_size();
        this.add_env(&mut env.borrow_mut(), 2, x + 3, 0);
        let (env_width, _) = env.borrow().get_size();
        let (_, y) = this.window.get_size();
        this.add_child(env, 1, y + 1);

        let lfo: ContainerRef<ParamId> = Rc::new(RefCell::new(Container::new()));
        lfo.borrow_mut().enable_border(true);
        this.add_lfo(&mut lfo.borrow_mut(), 1, 1, 0);
        let (x, _) = lfo.borrow().get_size();
        this.add_lfo(&mut lfo.borrow_mut(), 2, x + 2, 0);
        let (x, _) = lfo.borrow().get_size();
        this.add_lfo(&mut lfo.borrow_mut(), 3, x + 2, 0);
        let (lfo_width, _) = lfo.borrow().get_size();
        this.add_child(lfo, env_width + 2, y + 1);

        let sysinfo: ContainerRef<ParamId> = Rc::new(RefCell::new(Container::new()));
        sysinfo.borrow_mut().enable_border(true);
        this.add_sysinfo(&mut sysinfo.borrow_mut(), 1, 0);
        this.add_child(sysinfo, env_width + lfo_width + 4, y + 1);

        this.window.set_position(1, 1);
        this.window.set_color_scheme(this.colors.clone());
        this
    }

    pub fn add_child<C>(&mut self, child: Rc<RefCell<C>>, pos_x: Index, pos_y: Index)
        where C: Widget<ParamId> + 'static
    {
        self.window.add_child(child, pos_x, pos_y);
    }

    pub fn set_position(&mut self, x: Index, y: Index) {
        self.window.set_position(x, y);
    }

    pub fn set_dirty(&mut self, is_dirty: bool) {
        self.window.set_dirty(is_dirty);
    }
    pub fn get_size(&self) -> (Index, Index) {
        self.window.get_size()
    }

    pub fn draw(&mut self) {
        self.window.draw();
        print!("{}{}", color::Bg(self.colors.bg_light),
                       color::Fg(self.colors.fg_dark));
        self.window.set_dirty(false);
    }

    pub fn update_value(&mut self, key: &ParamId, value: Value) {
        self.controller.update(key, value);
    }

    pub fn handle_event(&mut self, msg: &UiMessage) {
        //self.mouse_handler.handle_event(msg, &self.window, &self.controller);
    }

    fn new_mod_dial_float(&mut self,
                          label: &'static str,
                          min: f64,
                          max: f64,
                          value: f64,
                          log: bool,
                          key: &ParamId) -> ContainerRef<ParamId> {
        let mut c = Container::new();
        let label = Label::new(label.to_string(), 10);
        let dial = Dial::new(Value::Float(min), Value::Float(max), Value::Float(value));
        dial.borrow_mut().set_logarithmic(log);
        dial.borrow_mut().set_key(key);
        let modul = Bar::new(Value::Float(0.0), Value::Float(100.0), Value::Float(0.0));
        self.controller.add_observer(key, dial.clone());
        self.mod_targets.insert(*key, modul.clone());
        c.add_child(label, 0, 1);
        c.add_child(dial, 9, 1);
        c.add_child(modul, 0, 2);
        Rc::new(RefCell::new(c))
    }

    fn new_mod_dial_int(&mut self,
                        label: &'static str,
                        min: i64,
                        max: i64,
                        value: i64,
                        log: bool,
                        key: &ParamId) -> ContainerRef<ParamId> {
        let mut c = Container::new();
        let label = Label::new(label.to_string(), 8);
        let dial = Dial::new(Value::Int(min), Value::Int(max), Value::Int(value));
        dial.borrow_mut().set_logarithmic(log);
        dial.borrow_mut().set_key(key);
        let modul = Bar::new(Value::Float(0.0), Value::Float(100.0), Value::Float(0.0));
        self.controller.add_observer(key, dial.clone());
        self.mod_targets.insert(*key, modul.clone());
        c.add_child(label, 0, 1);
        c.add_child(dial, 9, 1);
        c.add_child(modul, 0, 2);
        Rc::new(RefCell::new(c))
    }

    fn new_mod_slider_float(&mut self,
                            label: &'static str,
                            min: f64,
                            max: f64,
                            value: f64,
                            log: bool,
                            key: &ParamId) -> ContainerRef<ParamId> {
        // TODO: Add vertical modulation indicator
        let mut c = Container::new();
        let len = label.len() as Index;
        let label = Label::new(label.to_string(), len);
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

    fn new_label_value(&mut self,
                       label: &'static str,
                       value: i64,
                       key: &ParamId) -> ContainerRef<ParamId> {
        let mut c = Container::new();
        let len = label.len() as Index;
        let label = Label::new(label.to_string(), len);
        let val_display = ValueDisplay::new(Value::Int(value));
        self.controller.add_observer(key, val_display.clone());
        c.add_child(label, 0, 1);
        c.add_child(val_display, len + 1, 1);
        Rc::new(RefCell::new(c))
    }

    fn new_option(&mut self,
                  label: &'static str,
                  status: i64,
                  key: &ParamId) -> ContainerRef<ParamId> {
        let mut c = Container::new();
        let label = Label::new(label.to_string(), 8);
        let button = Button::new(Value::Int(status));
        self.controller.add_observer(key, button.clone());
        c.add_child(label, 0, 0);
        c.add_child(button, 10, 0);
        Rc::new(RefCell::new(c))
    }

    fn add_multi_osc(&mut self,
                     target: &mut Container<ParamId>,
                     func_id: usize,
                     x_offset: Index,
                     y_offset: Index) {
        let mut title = "Oscillator ".to_string();
        title.push(((func_id as u8) + '0' as u8) as char);
        let len = title.len();
        let title = Label::new(title, len as Index);
        target.add_child(title, 10 + x_offset, y_offset);

        let mut key = ParamId::new(Parameter::Oscillator, func_id, Parameter::Waveform);
        let osc_wave = self.new_mod_dial_int("Waveform", 0, 4, 0, false, &key);
        target.add_child(osc_wave, x_offset, 1 + y_offset);

        key.set(Parameter::Oscillator, func_id, Parameter::Level);
        let osc_level = self.new_mod_dial_float("Level", 0.0, 100.0, 0.0, false, &key);
        target.add_child(osc_level, x_offset, 4 + y_offset);

        key.set(Parameter::Oscillator, func_id, Parameter::Frequency);
        let osc_freq = self.new_mod_dial_int("Pitch", -24, 24, 0, false, &key);
        target.add_child(osc_freq, x_offset, 7 + y_offset);

        key.set(Parameter::Oscillator, func_id, Parameter::Blend);
        let osc_blend = self.new_mod_dial_float("Blend", 0.0, 5.0, 0.0, false, &key);
        target.add_child(osc_blend, x_offset, 10 + y_offset);

        key.set(Parameter::Oscillator, func_id, Parameter::Phase);
        let osc_phase = self.new_mod_dial_float("Phase", 0.0, 1.0, 0.0, false, &key);
        target.add_child(osc_phase, 14 + x_offset, 1 + y_offset);

        key.set(Parameter::Oscillator, func_id, Parameter::Voices);
        let osc_voices = self.new_mod_dial_int("Voices", 1, 7, 1, false, &key);
        target.add_child(osc_voices, 14 + x_offset, 4 + y_offset);

        key.set(Parameter::Oscillator, func_id, Parameter::Spread);
        let osc_spread = self.new_mod_dial_float("Spread", 0.0, 2.0, 0.0, false, &key);
        target.add_child(osc_spread, 14 + x_offset, 7 + y_offset);

        key.set(Parameter::Oscillator, func_id, Parameter::KeyFollow);
        let osc_sync = self.new_option("KeyFollow", 0, &key);
        target.add_child(osc_sync, 14 + x_offset, 11 + y_offset);

        if func_id == 2 {
            key.set(Parameter::Oscillator, func_id, Parameter::Sync);
            let osc_sync = self.new_option("Sync", 0, &key);
            target.add_child(osc_sync, x_offset, 13 + y_offset);
        }
    }

    fn add_env(&mut self,
               target: &mut Container<ParamId>,
               func_id: usize,
               x_offset: Index,
               y_offset: Index) {
        let mut title = "Envelope ".to_string();
        title.push(((func_id as u8) + '0' as u8) as char);
        let len = title.len();
        let title = Label::new(title, len as Index);
        target.add_child(title, x_offset, y_offset);

        let mut key = ParamId::new(Parameter::Envelope, func_id, Parameter::Attack);
        let env_attack = self.new_mod_slider_float("A", 0.0, 4000.0, 0.0, true, &key);
        target.add_child(env_attack, x_offset, 1 + y_offset);

        key.set(Parameter::Envelope, func_id, Parameter::Decay);
        let env_decay = self.new_mod_slider_float("D", 0.0, 4000.0, 0.0, true, &key);
        target.add_child(env_decay, 4 + x_offset, 1 + y_offset);

        key.set(Parameter::Envelope, func_id, Parameter::Sustain);
        let env_sustain = self.new_mod_slider_float("S", 0.0, 100.0, 0.0, false, &key);
        target.add_child(env_sustain, 8 + x_offset, 1 + y_offset);

        key.set(Parameter::Envelope, func_id, Parameter::Release);
        let env_release = self.new_mod_slider_float("R", 0.0, 8000.0, 0.0, true, &key);
        target.add_child(env_release, 12 + x_offset, 1 + y_offset);
    }

    fn add_lfo(&mut self,
               target: &mut Container<ParamId>,
               func_id: usize,
               x_offset: Index,
               y_offset: Index) {
        let mut title = "LFO ".to_string();
        title.push(((func_id as u8) + '0' as u8) as char);
        let len = title.len();
        let title = Label::new(title, len as Index);
        target.add_child(title, x_offset, y_offset);

        let mut key = ParamId::new(Parameter::Oscillator, func_id, Parameter::Waveform);
        let osc_wave = self.new_mod_dial_int("Waveform", 0, 4, 0, false, &key);
        target.add_child(osc_wave, x_offset, 1 + y_offset);

        key.set(Parameter::Oscillator, func_id, Parameter::Frequency);
        let osc_freq = self.new_mod_dial_int("Speed", -24, 24, 0, false, &key);
        target.add_child(osc_freq, x_offset, 4 + y_offset);
    }

    fn add_sysinfo(&mut self,
                   target: &mut Container<ParamId>,
                   x_offset: Index,
                   y_offset: Index) {
        let title = "System";
        let len = title.len();
        let title = Label::new(title.to_string(), len as Index);
        target.add_child(title, x_offset, y_offset);

        let mut key = ParamId::new(Parameter::System, 0, Parameter::Busy);
        let busy_value = self.new_label_value("Busy", 0, &key);
        target.add_child(busy_value, x_offset, 1 + y_offset);

        key.set(Parameter::System, 0, Parameter::Idle);
        let idle_value = self.new_label_value("Idle", 0, &key);
        target.add_child(idle_value, x_offset, 2 + y_offset);
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
            let param = SynthParam::new(key.function,
                                        key.function_id,
                                        key.parameter,
                                        ParameterValue::NoValue);
            let value = sound.get_value(&param);
            if let ParameterValue::NoValue = value { continue; };
            let value = Surface::param_to_widget_value(&value);
            item.borrow_mut().update(value);
        }
    }
}

