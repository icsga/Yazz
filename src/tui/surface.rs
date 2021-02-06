// Implements the control surface of the synth
use std::collections::HashMap;
use std::cell::RefCell;
use std::rc::Rc;

use super::{Parameter, ParamId, ParameterValue, SoundData, UiMessage};
use super::{Bar, Button, Canvas, CanvasRef, Container, ContainerRef, Controller,
            Dial, Index, Label, MouseHandler, ObserverRef, Printer ,ColorScheme,
            Slider, Value, ValueDisplay, Widget};

use super::container::{
JOIN_NONE, JOIN_LEFT, JOIN_RIGHT, JOIN_UP, JOIN_DOWN,
JOIN_LEFT_UP, JOIN_RIGHT_UP, JOIN_LEFT_DOWN, JOIN_RIGHT_DOWN,
MASK_LEFT_UP, MASK_RIGHT_UP, MASK_LEFT_DOWN, MASK_RIGHT_DOWN,
};

pub struct Surface {
    window: Container<ParamId>,
    controller: Controller<ParamId>,
    mod_targets: HashMap<ParamId, ObserverRef>, // Maps the modulation indicator to the corresponding parameter key
    mouse_handler: MouseHandler<ParamId>,
    pub canvas: CanvasRef<ParamId>,
}

impl Surface {
    pub fn new(colors: Rc<ColorScheme>) -> Surface {
        let window = Container::new();
        let controller = Controller::new();
        let mod_targets: HashMap<ParamId, ObserverRef> = HashMap::new();
        let mouse_handler = MouseHandler::new();
        let canvas: CanvasRef<ParamId> = Canvas::new(50, 21);
        let canvas_clone = canvas.clone();
        let mut this = Surface{window,
                               controller,
                               mod_targets,
                               mouse_handler,
                               canvas};

        let osc: ContainerRef<ParamId> = Rc::new(RefCell::new(Container::new()));
        this.window.enable_border(true);
        this.add_multi_osc(&mut osc.borrow_mut(), 1, 0, 0);
        this.add_multi_osc(&mut osc.borrow_mut(), 2, 32, 0);
        this.add_multi_osc(&mut osc.borrow_mut(), 3, 65, 0);
        let (_, osc_height) = osc.borrow().get_size();
        this.add_child(osc, 1, 0);

        let env: ContainerRef<ParamId> = Rc::new(RefCell::new(Container::new()));
        env.borrow_mut().enable_border(true);
        env.borrow_mut().join_border(JOIN_NONE, JOIN_RIGHT, JOIN_NONE, JOIN_RIGHT | JOIN_DOWN);
        this.add_env(&mut env.borrow_mut(), 1, 1, 0);
        let x = env.borrow().get_width();
        this.add_env(&mut env.borrow_mut(), 2, x + 3, 0);
        let x = env.borrow().get_width();
        this.add_env(&mut env.borrow_mut(), 3, x + 3, 0);
        let (env_width, env_height) = env.borrow().get_size();
        this.add_child(env, 1, osc_height);

        let lfo: ContainerRef<ParamId> = Rc::new(RefCell::new(Container::new()));
        lfo.borrow_mut().enable_border(true);
        lfo.borrow_mut().join_border(JOIN_LEFT, JOIN_NONE, JOIN_LEFT | JOIN_DOWN, JOIN_DOWN);
        this.add_lfo(&mut lfo.borrow_mut(), 1, 1, 0);
        let x = lfo.borrow().get_width();
        this.add_lfo(&mut lfo.borrow_mut(), 2, x, 0);

        this.add_glfo(&mut lfo.borrow_mut(), 1, x * 2 - 0, 0);
        this.add_glfo(&mut lfo.borrow_mut(), 2, x * 3 - 1, 0);

        let (_, lfo_height) = lfo.borrow().get_size();
        this.add_child(lfo, env_width + 1, osc_height);

        this.add_child(canvas_clone, 2, osc_height + env_height);

        let filter: ContainerRef<ParamId> = Rc::new(RefCell::new(Container::new()));
        filter.borrow_mut().enable_border(true);
        filter.borrow_mut().join_border(JOIN_LEFT | JOIN_UP, JOIN_UP, JOIN_DOWN, JOIN_DOWN);
        this.add_filter(&mut filter.borrow_mut(), 1, 1, 0);
        let (filter_width, filter_height) = filter.borrow().get_size();
        this.add_filter(&mut filter.borrow_mut(), 2, filter_width + 1, 0);
        this.add_child(filter, env_width + 1, osc_height + lfo_height - 1);

        let delay: ContainerRef<ParamId> = Rc::new(RefCell::new(Container::new()));
        delay.borrow_mut().enable_border(true);
        delay.borrow_mut().join_border(JOIN_UP, JOIN_RIGHT, JOIN_NONE, JOIN_RIGHT);
        this.add_delay(&mut delay.borrow_mut(), 1, 0);
        let (delay_width, _delay_height) = delay.borrow().get_size();
        this.add_child(delay, env_width + 1, osc_height + lfo_height + filter_height - 2);

        let patch: ContainerRef<ParamId> = Rc::new(RefCell::new(Container::new()));
        patch.borrow_mut().enable_border(true);
        patch.borrow_mut().join_border(JOIN_LEFT, JOIN_UP, JOIN_LEFT, JOIN_NONE);
        this.add_patch(&mut patch.borrow_mut(), 1, 0);
        this.add_child(patch, env_width + delay_width + 1, osc_height + lfo_height + filter_height - 2);

        let sysinfo: ContainerRef<ParamId> = Rc::new(RefCell::new(Container::new()));
        sysinfo.borrow_mut().enable_border(true);
        this.add_sysinfo(&mut sysinfo.borrow_mut(), 1, 0);
        this.add_child(sysinfo, 95, 0);

        this.window.set_position(1, 1);
        this.window.set_color_scheme(colors);
        this
    }

    pub fn set_color_scheme(&mut self, colors: Rc<ColorScheme>) {
        self.window.set_color_scheme(colors);
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

    pub fn draw(&mut self, p: &mut dyn Printer) {
        self.window.draw(p);
        self.window.set_dirty(false);
    }

    pub fn update_value(&mut self, key: &ParamId, value: Value) {
        self.controller.update(key, value);
    }

    pub fn handle_event(&mut self, _msg: &UiMessage) {
        //self.mouse_handler.handle_event(msg, &self.window, &self.controller);
    }

    fn new_mod_dial_float(&mut self,
                          label: &str,
                          min: f64,
                          max: f64,
                          value: f64,
                          log: bool,
                          key: &ParamId) -> ContainerRef<ParamId> {
        let mut c = Container::new();
        let label = Label::new(label.to_string(), 10);
        let dial = Dial::new(Value::Float(min), Value::Float(max), Value::Float(value));
        dial.borrow_mut().set_logarithmic(log);
        dial.borrow_mut().set_key(*key);
        let modul = Bar::new(Value::Float(0.0), Value::Float(100.0), Value::Float(0.0));
        self.controller.add_observer(key, dial.clone());
        self.mod_targets.insert(*key, modul.clone());
        c.add_child(label, 0, 1);
        c.add_child(dial, 10, 1);
        c.add_child(modul, 0, 2);
        Rc::new(RefCell::new(c))
    }

    fn new_mod_dial_int(&mut self,
                        label: &str,
                        min: i64,
                        max: i64,
                        value: i64,
                        log: bool,
                        key: &ParamId) -> ContainerRef<ParamId> {
        let mut c = Container::new();
        let label = Label::new(label.to_string(), 10);
        let dial = Dial::new(Value::Int(min), Value::Int(max), Value::Int(value));
        dial.borrow_mut().set_logarithmic(log);
        dial.borrow_mut().set_key(*key);
        let modul = Bar::new(Value::Float(0.0), Value::Float(100.0), Value::Float(0.0));
        self.controller.add_observer(key, dial.clone());
        self.mod_targets.insert(*key, modul.clone());
        c.add_child(label, 0, 1);
        c.add_child(dial, 10, 1);
        c.add_child(modul, 0, 2);
        Rc::new(RefCell::new(c))
    }

    fn new_mod_slider_float(&mut self,
                            label: &str,
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

    fn new_label_value_int(&mut self,
                           label: &str,
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

    fn new_label_value_float(&mut self,
                            label: &str,
                            value: f64,
                            key: &ParamId) -> ContainerRef<ParamId> {
        let mut c = Container::new();
        let len = label.len() as Index;
        let label = Label::new(label.to_string(), len);
        let val_display = ValueDisplay::new(Value::Float(value));
        self.controller.add_observer(key, val_display.clone());
        c.add_child(label, 0, 1);
        c.add_child(val_display, len + 1, 1);
        Rc::new(RefCell::new(c))
    }

    fn new_textfield(&mut self,
                     label: &str,
                     text: &str,
                     key: &ParamId) -> ContainerRef<ParamId> {
        let mut c = Container::new();
        let label = Label::new(label.to_string(), 10);
        let value = Label::new(text.to_string(), 15);
        self.controller.add_observer(key, value.clone());
        c.add_child(label, 0, 1);
        c.add_child(value, 0, 11);
        Rc::new(RefCell::new(c))
    }

    fn new_option(&mut self,
                  label: &str,
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
        title.push(((func_id as u8) + b'0') as char);
        let len = title.len();
        let title = Label::new(title, len as Index);
        //title.borrow_mut().select_light();
        target.add_child(title, 10 + x_offset, y_offset);

        let mut key = ParamId::new(Parameter::Oscillator, func_id, Parameter::Level);
        let osc_level = self.new_mod_dial_float("Level", 0.0, 100.0, 0.0, false, &key);
        target.add_child(osc_level, x_offset, 1 + y_offset);

        key.set(Parameter::Oscillator, func_id, Parameter::Voices);
        let osc_voices = self.new_mod_dial_int("Voices", 1, 7, 1, false, &key);
        target.add_child(osc_voices, 14 + x_offset, 1 + y_offset);

        key.set(Parameter::Oscillator, func_id, Parameter::WaveIndex);
        let osc_wave_id = self.new_mod_dial_float("Waveindex", 0.0, 1.0, 0.0, false, &key);
        target.add_child(osc_wave_id, x_offset, 7 + y_offset);

        key.set(Parameter::Oscillator, func_id, Parameter::Tune);
        let osc_freq = self.new_mod_dial_int("Pitch", -24, 24, 0, false, &key);
        target.add_child(osc_freq, x_offset, 4 + y_offset);

        key.set(Parameter::Oscillator, func_id, Parameter::Spread);
        let osc_spread = self.new_mod_dial_float("FrqSpread", 0.0, 2.0, 0.0, false, &key);
        target.add_child(osc_spread, 14 + x_offset, 4 + y_offset);

        key.set(Parameter::Oscillator, func_id, Parameter::KeyFollow);
        let key_follow = self.new_option("KeyFollow", 0, &key);
        target.add_child(key_follow, 14 + x_offset, 8 + y_offset);

        /*
        key.set(Parameter::Oscillator, func_id, Parameter::Wavetable);
        let wavetable = self.new_textfield("Wavetable", "default", &key);
        target.add_child(wavetable, 14 + x_offset, 10 + y_offset);
        */

        if func_id == 2 {
            key.set(Parameter::Oscillator, func_id, Parameter::Sync);
            let osc_sync = self.new_option("OscSync", 0, &key);
            target.add_child(osc_sync, 14 + x_offset, 9 + y_offset);
        }
    }

    fn add_env(&mut self,
               target: &mut Container<ParamId>,
               func_id: usize,
               x_offset: Index,
               y_offset: Index) {
        let mut title = "Envelope ".to_string();
        title.push(((func_id as u8) + b'0') as char);
        let len = title.len();
        let title = Label::new(title, len as Index);
        target.add_child(title, x_offset, y_offset);

        let mut key = ParamId::new(Parameter::Envelope, func_id, Parameter::Delay);
        let env_delay = self.new_mod_slider_float("D", 0.0, 4000.0, 0.0, true, &key);
        target.add_child(env_delay, x_offset, 1 + y_offset);

        key = ParamId::new(Parameter::Envelope, func_id, Parameter::Attack);
        let env_attack = self.new_mod_slider_float("A", 0.0, 4000.0, 0.0, true, &key);
        target.add_child(env_attack, 3 + x_offset, 1 + y_offset);

        key.set(Parameter::Envelope, func_id, Parameter::Decay);
        let env_decay = self.new_mod_slider_float("D", 0.0, 4000.0, 0.0, true, &key);
        target.add_child(env_decay, 6 + x_offset, 1 + y_offset);

        key.set(Parameter::Envelope, func_id, Parameter::Sustain);
        let env_sustain = self.new_mod_slider_float("S", 0.0, 1.0, 0.0, false, &key);
        target.add_child(env_sustain, 9 + x_offset, 1 + y_offset);

        key.set(Parameter::Envelope, func_id, Parameter::Release);
        let env_release = self.new_mod_slider_float("R", 0.0, 8000.0, 0.0, true, &key);
        target.add_child(env_release, 12 + x_offset, 1 + y_offset);

        key.set(Parameter::Envelope, func_id, Parameter::Loop);
        let env_loop = self.new_option("Loop", 0, &key);
        target.add_child(env_loop, x_offset, 9 + y_offset);
    }

    fn add_lfo(&mut self,
               target: &mut Container<ParamId>,
               func_id: usize,
               x_offset: Index,
               y_offset: Index) {
        let mut title = "LFO ".to_string();
        title.push(((func_id as u8) + b'0') as char);
        let len = title.len();
        let title = Label::new(title, len as Index);
        target.add_child(title, x_offset, y_offset);

        let mut key = ParamId::new(Parameter::Lfo, func_id, Parameter::Waveform);
        let lfo_wave = self.new_mod_dial_int("Waveform", 0, 5, 0, false, &key);
        target.add_child(lfo_wave, x_offset, 1 + y_offset);

        key.set(Parameter::Lfo, func_id, Parameter::Frequency);
        let lfo_freq = self.new_mod_dial_float("Speed", 0.0, 44.1, 0.01, false, &key);
        target.add_child(lfo_freq, x_offset, 4 + y_offset);

        key.set(Parameter::Lfo, func_id, Parameter::Amount);
        let lfo_amount = self.new_mod_dial_float("Amount", 0.0, 1.0, 0.01, false, &key);
        target.add_child(lfo_amount, x_offset, 7 + y_offset);
    }

    fn add_glfo(&mut self,
               target: &mut Container<ParamId>,
               func_id: usize,
               x_offset: Index,
               y_offset: Index) {
        let mut title = "Global LFO ".to_string();
        title.push(((func_id as u8) + b'0') as char);
        let len = title.len();
        let title = Label::new(title, len as Index);
        target.add_child(title, x_offset, y_offset);

        let mut key = ParamId::new(Parameter::GlobalLfo, func_id, Parameter::Waveform);
        let glfo_wave = self.new_mod_dial_int("Waveform", 0, 5, 0, false, &key);
        target.add_child(glfo_wave, x_offset, 1 + y_offset);

        key.set(Parameter::GlobalLfo, func_id, Parameter::Frequency);
        let glfo_freq = self.new_mod_dial_float("Speed", 0.0, 44.1, 0.01, false, &key);
        target.add_child(glfo_freq, x_offset, 4 + y_offset);

        key.set(Parameter::GlobalLfo, func_id, Parameter::Amount);
        let glfo_amount = self.new_mod_dial_float("Amount", 0.0, 1.0, 0.01, false, &key);
        target.add_child(glfo_amount, x_offset, 7 + y_offset);
    }

    fn add_filter(&mut self,
                  target: &mut Container<ParamId>,
                  func_id: usize,
                  x_offset: Index,
                  y_offset: Index) {
        let mut title = "Filter ".to_string();
        title.push(((func_id as u8) + b'0') as char);
        let len = title.len();
        let title = Label::new(title, len as Index);
        target.add_child(title, x_offset, y_offset);

        let mut key = ParamId::new(Parameter::Filter, func_id, Parameter::Cutoff);
        let filter_cutoff = self.new_mod_dial_float("Cutoff", 1.0, 8000.0, 2000.0, false, &key);
        target.add_child(filter_cutoff, x_offset, 1 + y_offset);

        key.set(Parameter::Filter, func_id, Parameter::Resonance);
        let filter_reso = self.new_mod_dial_float("Resonance", 0.0, 1.0, 0.5, false, &key);
        target.add_child(filter_reso, 14 + x_offset, 1 + y_offset);

        key.set(Parameter::Filter, func_id, Parameter::Gain);
        let filter_gain = self.new_mod_dial_float("Gain", 0.0, 2.0, 1.0, false, &key);
        target.add_child(filter_gain, x_offset, 4 + y_offset);

        key.set(Parameter::Filter, func_id, Parameter::EnvDepth);
        let filter_edepth = self.new_mod_dial_float("EnvDepth", 0.0, 1.0, 0.0, false, &key);
        target.add_child(filter_edepth, 14 + x_offset, 4 + y_offset);

        key.set(Parameter::Filter, func_id, Parameter::KeyFollow);
        let filter_follow = self.new_option("KeyFollow", 0, &key);
        target.add_child(filter_follow, x_offset, 8 + y_offset);
    }

    fn add_delay(&mut self,
                 target: &mut Container<ParamId>,
                 x_offset: Index,
                 y_offset: Index) {
        let title = "Delay";
        let len = title.len();
        let title = Label::new(title.to_string(), len as Index);
        target.add_child(title, x_offset, y_offset);

        let mut key = ParamId::new(Parameter::Delay, 1, Parameter::Time);
        let time = self.new_mod_dial_float("Time", 0.01, 1.0, 0.5, false, &key);
        target.add_child(time, x_offset, 1 + y_offset);

        key.set(Parameter::Delay, 1, Parameter::Level);
        let level = self.new_mod_dial_float("Level", 0.0, 1.0, 0.5, false, &key);
        target.add_child(level, 14 + x_offset, 1 + y_offset);

        key.set(Parameter::Delay, 1, Parameter::Feedback);
        let feedback = self.new_mod_dial_float("Feedback", 0.0, 1.0, 0.5, false, &key);
        target.add_child(feedback, x_offset, 4 + y_offset);

        key.set(Parameter::Delay, 1, Parameter::Tone);
        let tone = self.new_mod_dial_float("Tone", 100.0, 5000.0, 3000.0, false, &key);
        target.add_child(tone, 14 + x_offset, 4 + y_offset);

        key.set(Parameter::Delay, 1, Parameter::Sync);
        let sync = self.new_option("BpmSync", 0, &key);
        target.add_child(sync, x_offset, 8 + y_offset);
    }

    fn add_patch(&mut self,
                 target: &mut Container<ParamId>,
                 x_offset: Index,
                 y_offset: Index) {
        let title = "Patch";
        let len = title.len();
        let title = Label::new(title.to_string(), len as Index);
        target.add_child(title, x_offset, y_offset);

        // Level, Drive, Voices, Spread

        let mut key = ParamId::new(Parameter::Patch, 1, Parameter::Level);
        let level = self.new_mod_dial_float("Level", 0.0, 100.0, 50.0, false, &key);
        target.add_child(level, x_offset, 1 + y_offset);

        key.set(Parameter::Patch, 1, Parameter::Drive);
        let drive = self.new_mod_dial_float("Drive", 0.0, 10.0, 5.0, false, &key);
        target.add_child(drive, 14 + x_offset, 1 + y_offset);

        key.set(Parameter::Patch, 1, Parameter::Voices);
        let voices = self.new_mod_dial_int("Voices", 1, 10, 1, false, &key);
        target.add_child(voices, x_offset, 4 + y_offset);

        key.set(Parameter::Patch, 1, Parameter::Spread);
        let spread = self.new_mod_dial_float("PanSpread", 0.0, 1.0, 1.0, false, &key);
        target.add_child(spread, 14 + x_offset, 4 + y_offset);

        key.set(Parameter::Patch, 1, Parameter::Bpm);
        let bpm = self.new_label_value_float("Bpm", 120.0, &key);
        target.add_child(bpm, x_offset, 7 + y_offset);
    }

    fn add_sysinfo(&mut self,
                   target: &mut Container<ParamId>,
                   x_offset: Index,
                   y_offset: Index) {
        //let title = "System";
        //let len = title.len();
        //let title = Label::new(title.to_string(), len as Index);
        //target.add_child(title, x_offset, y_offset);

        let mut key = ParamId::new(Parameter::System, 0, Parameter::Busy);
        let busy_value = self.new_label_value_int("Busy", 0, &key);
        target.add_child(busy_value, x_offset, y_offset - 1);

        key.set(Parameter::System, 0, Parameter::Idle);
        let idle_value = self.new_label_value_int("Idle", 0, &key);
        target.add_child(idle_value, x_offset, y_offset);
    }

    fn param_to_widget_value(value: ParameterValue) -> Value {
        match value {
            ParameterValue::Int(v) => Value::Int(v),
            ParameterValue::Float(v) => Value::Float(v),
            ParameterValue::Choice(v) => Value::Int(v as i64),
            _ => panic!(),
        }
    }

    pub fn update_all(&mut self, sound: &SoundData) {
        for (key, item) in self.controller.observers.iter_mut() {
            let param = ParamId::new(key.function, key.function_id, key.parameter);
            let value = sound.get_value(&param);
            if let ParameterValue::NoValue = value {
                continue;
            }
            let value = Surface::param_to_widget_value(value);
            item.borrow_mut().update(value);
        }
    }

    pub fn set_sound_info(&mut self, program: usize, name: &str) {
        let name = (program + 1).to_string() + ": " + name;
        self.window.set_title(name);
    }
}

