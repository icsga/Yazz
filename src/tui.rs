extern crate term_cursor as cursor;

use super::parameter::{Parameter, ParameterValue, SynthParam};
use super::midi_handler::{MidiMessage, MessageType};
use super::TermionWrapper;
use super::{UiMessage, SynthMessage};
use super::canvas::Canvas;

use termion::clear;
use termion::event::Key;
//use termion::cursor::{DetectCursorPos, Goto};
use termion::color;
use termion::color::{Black, White, LightWhite, Reset, Rgb};
use std::io::{stdout, stdin};
use std::convert::TryInto;
use std::num::ParseFloatError;
use std::io;
use std::io::Write;

use crossbeam_channel::unbounded;
use crossbeam_channel::{Sender, Receiver};

use std::fmt::{self, Debug, Display};
use std::thread::spawn;

#[derive(Copy, Clone, Debug, PartialEq)]
enum TuiState {
    Init,
    Function,
    FunctionIndex,
    Param,
    Value,
    EventComplete
}

impl fmt::Display for TuiState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

fn next(current: &TuiState) -> TuiState {
    use TuiState::*;
    match *current {
        Init => Function,
        Function => FunctionIndex,
        FunctionIndex => Param,
        Param => Value,
        Value => EventComplete,
        EventComplete => Function
    }
}

fn previous(current: &TuiState) -> TuiState {
    use TuiState::*;
    match *current {
        Init => Init,
        Function => Function,
        FunctionIndex => Function,
        Param => FunctionIndex,
        Value => Param,
        EventComplete => Function
    }
}

#[derive(Debug)]
enum ValueRange {
    IntRange(i64, i64),
    FloatRange(f32, f32),
    ChoiceRange(&'static [Selection]),
    NoRange
}

/* Item for a list of selectable functions */
#[derive(Debug)]
struct Selection {
    item: Parameter,
    key: Key,
    val_range: ValueRange,
    next: &'static [Selection]
}

static FUNCTIONS: [Selection; 4] = [
    Selection{item: Parameter::Oscillator, key: Key::Char('o'), val_range: ValueRange::IntRange(1, 3), next: &OSC_PARAMS},
    Selection{item: Parameter::Lfo,        key: Key::Char('l'), val_range: ValueRange::IntRange(1, 3), next: &LFO_PARAMS},
    Selection{item: Parameter::Filter,     key: Key::Char('f'), val_range: ValueRange::IntRange(1, 2), next: &FILTER_PARAMS},
    Selection{item: Parameter::Envelope,   key: Key::Char('e'), val_range: ValueRange::IntRange(1, 2), next: &ENV_PARAMS},
];

static OSC_PARAMS: [Selection; 6] = [
    Selection{item: Parameter::Waveform,  key: Key::Char('w'), val_range: ValueRange::ChoiceRange(&WAVEFORM), next: &[]},
    Selection{item: Parameter::Level,     key: Key::Char('l'), val_range: ValueRange::FloatRange(0.0, 100.0), next: &[]},
    Selection{item: Parameter::Frequency, key: Key::Char('f'), val_range: ValueRange::IntRange(-24, 24), next: &[]},
    Selection{item: Parameter::Blend,     key: Key::Char('b'), val_range: ValueRange::FloatRange(0.0, 5.0), next: &[]},
    Selection{item: Parameter::Phase,     key: Key::Char('p'), val_range: ValueRange::FloatRange(0.0, 1.0), next: &[]},
    Selection{item: Parameter::Sync,      key: Key::Char('s'), val_range: ValueRange::IntRange(0, 1), next: &[]},
];

static LFO_PARAMS: [Selection; 3] = [
    Selection{item: Parameter::Waveform,  key: Key::Char('w'), val_range: ValueRange::IntRange(1, 3), next: &[]},
    Selection{item: Parameter::Frequency, key: Key::Char('f'), val_range: ValueRange::FloatRange(0.0, 22000.0), next: &[]},
    Selection{item: Parameter::Phase,     key: Key::Char('p'), val_range: ValueRange::FloatRange(0.0, 100.0), next: &[]},
];

static FILTER_PARAMS: [Selection; 3] = [
    Selection{item: Parameter::Type,      key: Key::Char('t'), val_range: ValueRange::IntRange(1, 3), next: &[]},
    Selection{item: Parameter::FilterFreq,key: Key::Char('f'), val_range: ValueRange::FloatRange(0.0, 22000.0), next: &[]},
    Selection{item: Parameter::Resonance, key: Key::Char('r'), val_range: ValueRange::FloatRange(0.0, 100.0), next: &[]},
];

static ENV_PARAMS: [Selection; 4] = [
    Selection{item: Parameter::Attack,  key: Key::Char('a'), val_range: ValueRange::FloatRange(0.0, 1000.0), next: &[]}, // Value = Duration in ms
    Selection{item: Parameter::Decay,   key: Key::Char('d'), val_range: ValueRange::FloatRange(0.0, 1000.0), next: &[]},
    Selection{item: Parameter::Sustain, key: Key::Char('s'), val_range: ValueRange::FloatRange(0.0, 100.0), next: &[]},
    Selection{item: Parameter::Release, key: Key::Char('r'), val_range: ValueRange::FloatRange(0.0, 1000.0), next: &[]},
];

static WAVEFORM: [Selection; 5] = [
    Selection{item: Parameter::Sine,      key: Key::Char('s'), val_range: ValueRange::NoRange, next: &[]},
    Selection{item: Parameter::Triangle,  key: Key::Char('t'), val_range: ValueRange::NoRange, next: &[]},
    Selection{item: Parameter::Saw,       key: Key::Char('w'), val_range: ValueRange::NoRange, next: &[]},
    Selection{item: Parameter::Square,    key: Key::Char('q'), val_range: ValueRange::NoRange, next: &[]},
    Selection{item: Parameter::Noise ,    key: Key::Char('n'), val_range: ValueRange::NoRange, next: &[]},
];

struct SelectedItem {
    item_list: &'static [Selection], // The selection this item is coming from
    item_index: usize, // Index into the selection list
    value: ParameterValue, // ID or value of the selected item
}

pub struct Tui {
    // Function selection
    state: TuiState,
    sender: Sender<SynthMessage>,
    ui_receiver: Receiver<UiMessage>,

    // TUI handling
    current_list: &'static [Selection],
    selected_function: SelectedItem,
    selected_parameter: SelectedItem,

    canvas: Canvas,

    temp_string: String,
}

impl Tui {
    pub fn new(sender: Sender<SynthMessage>, ui_receiver: Receiver<UiMessage>) -> Tui {
        //let (x, y) = stdout().cursor_pos().unwrap();; //self.termion.cursor_pos().unwrap();
        let state = TuiState::Init;
        let current_list = &FUNCTIONS;
        let selected_function = SelectedItem{item_list: &FUNCTIONS, item_index: 0, value: ParameterValue::Int(1)};
        let selected_parameter = SelectedItem{item_list: &OSC_PARAMS, item_index: 0, value: ParameterValue::Int(1)};
        let temp_string = String::new();
        let canvas = Canvas::new(100, 30);
        Tui{state,
            sender,
            ui_receiver,
            current_list,
            selected_function,
            selected_parameter,
            canvas,
            temp_string
        }
    }

    pub fn run(mut tui: Tui) -> std::thread::JoinHandle<()> {
        let mut get_wave = true;
        let handler = spawn(move || {
            loop {
                get_wave = true;
                let msg = tui.ui_receiver.recv().unwrap();
                match msg {
                    UiMessage::Midi(m)  => tui.handle_midi(m),
                    UiMessage::Key(m) => tui.handle_input(m),
                    UiMessage::Param(m) => tui.handle_param(m),
                    UiMessage::WaveBuffer(m) => {
                        tui.handle_wavebuffer(m);
                        get_wave = false;
                    },
                };
                if get_wave {
                    tui.get_waveform();
                }
                tui.display();
            }
        });
        handler
    }

    pub fn init(&mut self) {
        print!("{}{}", clear::All, cursor::Goto(0, 0));
        self.temp_string.clear();
    }

    pub fn handle_midi(&mut self, m: MidiMessage) {
        match m.get_message_type() {
            MessageType::ControlChg => {
                if m.param == 0x01 {
                    // ModWheel
                    self.handle_control_change(m.value as i64);
                }
            },
            _ => ()
        }
    }

    fn handle_control_change(&mut self, val: i64) {
        match self.state {
            TuiState::Param => self.change_state(TuiState::Value),
            TuiState::Value => (),
            _ => return,
        }
        let item = &mut self.selected_parameter;
        match item.item_list[item.item_index].val_range {
            ValueRange::IntRange(min, max) => {
                let inc: f32 = (max - min) as f32 / 127.0;
                let value = min + (val as f32 * inc) as i64;
                Tui::update_value(item, &ParameterValue::Int(value), &mut self.temp_string);
            }
            ValueRange::FloatRange(min, max) => {
                let inc: f32 = (max - min) / 127.0;
                let value = min + val as f32 * inc;
                Tui::update_value(item, &ParameterValue::Float(value), &mut self.temp_string);
            }
            ValueRange::ChoiceRange(choice_list) => {
                let inc: f32 = choice_list.len() as f32 / 127.0;
                let value = (val as f32 * inc) as i64;
                Tui::update_value(item, &ParameterValue::Choice(value as usize), &mut self.temp_string);
            }
            _ => ()
        }
        self.send_event();
    }

    pub fn handle_param(&mut self, m: SynthParam) {
        let item = &mut self.selected_parameter;
        Tui::update_value(item, &m.value, &mut self.temp_string);
    }

    pub fn handle_wavebuffer(&mut self, m: Vec<f32>) {
        self.canvas.clear();
        for (x_pos, v) in m.iter().enumerate() {
            let y_pos = ((v + 1.0) * (29.0 / 2.0)) as usize;
            self.canvas.set(x_pos, y_pos, '∘');
        }
    }

    pub fn handle_input(&mut self, c: termion::event::Key) {
        let new_state = match self.state {
            TuiState::Init => {
                self.init();
                TuiState::Function
            }
            TuiState::Function => Tui::select_item(c, &mut self.selected_function, &self.state),
            TuiState::FunctionIndex => Tui::get_value(c, &mut self.selected_function, &self.state, &mut self.temp_string),
            TuiState::Param => Tui::select_item(c, &mut self.selected_parameter, &self.state),
            TuiState::Value => Tui::get_value(c, &mut self.selected_parameter, &self.state, &mut self.temp_string),
            TuiState::EventComplete => {
                self.init();
                TuiState::Function
            }
        };
        self.change_state(new_state);
    }

    fn change_state(&mut self, new_state: TuiState) {
        if new_state != self.state {
            self.state = new_state;
            match new_state {
                TuiState::Init => {}
                TuiState::Function => {}
                TuiState::FunctionIndex => {}
                TuiState::Param => {
                    self.selected_parameter.item_list = self.selected_function.item_list[self.selected_function.item_index].next;
                    Tui::select_param(&mut self.selected_parameter);
                }
                TuiState::Value => {}
                TuiState::EventComplete => {}
            }
        }
        if new_state == TuiState::Param {
            self.get_current_value();
        }
        if new_state == TuiState::Value {
            self.send_event();
        }
    }

    /** Queries the current value of the selected parameter, since we don't keep a local copy. */
    fn get_current_value(&self) {
        let function = &self.selected_function.item_list[self.selected_function.item_index];
        let function_id = if let ParameterValue::Int(x) = &self.selected_function.value { *x as usize } else { panic!() };
        let parameter = &self.selected_parameter.item_list[self.selected_parameter.item_index];
        let param_val = &self.selected_parameter.value;
        let param = SynthParam::new(function.item, function_id, parameter.item, *param_val);
        self.sender.send(SynthMessage::ParamQuery(param)).unwrap();
    }

    fn get_waveform(&self) {
        let buffer = vec!(0.0; 100);
        self.sender.send(SynthMessage::WaveBuffer(buffer)).unwrap();
    }

    fn select_item(c: termion::event::Key, item: &mut SelectedItem, state: &TuiState) -> TuiState {
        match c {
            Key::Up => {
                if item.item_index < item.item_list.len() - 1 {
                    item.item_index += 1;
                    Tui::select_param(item);
                }
                *state
            }
            Key::Down => {
                if item.item_index > 0 {
                    item.item_index -= 1;
                    Tui::select_param(item);
                }
                *state
            }
            Key::Left => previous(state),
            Key::Right => next(state),
            _ => {
                Tui::select_by_key(c, item, state)
            }
        }
    }

    fn select_by_key(c: termion::event::Key, item: &mut SelectedItem, state: &TuiState) -> TuiState {
        for (count, f) in item.item_list.iter().enumerate() {
            if f.key == c {
                item.item_index = count;
                Tui::select_param(item);
                return next(state);
            }
        }
        *state
    }

    fn get_value(c: termion::event::Key, item: &mut SelectedItem, state: &TuiState, temp_string: &mut String) -> TuiState {
        let val = item.value;
        match item.item_list[item.item_index].val_range {
            ValueRange::IntRange(min, max) => {
                let mut current = if let ParameterValue::Int(x) = val { x } else { panic!() };
                let new_state = match c {
                    Key::Char(x) => {
                        match x {
                            '0' | '1' | '2' | '3' | '4' | '5' | '6' | '7' | '8' | '9' => {
                                let y = x as i64 - '0' as i64;
                                let val_digit_added = current * 10 + y;
                                if val_digit_added > max {
                                    current = y; // Can't add another digit, replace current value with new one
                                } else {
                                    current = val_digit_added;
                                }
                                if y * 10 > max {
                                    next(state) // Can't add another digit, accept value as final and move on
                                } else {
                                    *state // Could add more digits, not finished yet
                                }
                            },
                            '\n' => next(state),
                            _ => *state,
                        }
                    }
                    Key::Up        => { current += 1; *state },
                    Key::Down      => { if current > 0 { current -= 1; } *state },
                    Key::Left => previous(state),
                    _ => TuiState::Param,
                };
                Tui::update_value(item, &ParameterValue::Int(current), temp_string);
                new_state
            }
            ValueRange::FloatRange(min, max) => {
                let mut current = if let ParameterValue::Float(x) = val { x } else { panic!() };
                let new_state = match c {
                    Key::Char(x) => {
                        match x {
                            '0' | '1' | '2' | '3' | '4' | '5' | '6' | '7' | '8' | '9' | '.' => {
                                temp_string.push(x);
                                let value: Result<f32, ParseFloatError> = temp_string.parse();
                                current = if let Ok(x) = value { x } else { current };
                                *state
                            },
                            '\n' => next(state),
                            _ => previous(state),
                        }
                    }
                    Key::Up        => { current += 1.0; *state },
                    Key::Down      => { current -= 1.0; *state },
                    Key::Left => previous(state),
                    Key::Backspace => {
                        let len = temp_string.len();
                        if len > 0 {
                            temp_string.pop();
                            if len >= 1 {
                                let value = temp_string.parse();
                                current = if let Ok(x) = value { x } else { current };
                                *state
                            } else {
                                current = 0.0;
                                previous(state)
                            }
                        } else {
                            previous(state)
                        }
                    }
                    _ => previous(state)
                };
                Tui::update_value(item, &ParameterValue::Float(current), temp_string);
                new_state
            }
            ValueRange::ChoiceRange(choice_list) => {
                let mut current = if let ParameterValue::Choice(x) = val { x } else { panic!() };
                let new_state = match c {
                    Key::Up        => {current += 1; *state },
                    Key::Down      => {if current > 0 { current -= 1 }; *state },
                    Key::Left => {
                        previous(state)
                    }
                    Key::Char('\n') => next(state),
                    _ => *state
                };
                Tui::update_value(item, &ParameterValue::Choice(current), temp_string);
                new_state
            }
            _ => TuiState::Value
        }
    }

    fn update_value(item: &mut SelectedItem, val: &ParameterValue, temp_string: &mut String) {
        match item.item_list[item.item_index].val_range {
            ValueRange::IntRange(min, max) => {
                let val = if let ParameterValue::Int(x) = *val { x } else { panic!(); };
                if val <= max && val >= min {
                    item.value = ParameterValue::Int(val.try_into().unwrap());
                }
            }
            ValueRange::FloatRange(min, max) => {
                let mut val = if let ParameterValue::Float(x) = *val { x } else { panic!(); };
                let has_period =  temp_string.contains(".");
                if val > max {
                    val = max;
                }
                if val < min {
                    val = min;
                }
                temp_string.clear();
                temp_string.push_str(val.to_string().as_str());
                if !temp_string.contains(".") && has_period {
                    temp_string.push('.');
                }
                item.value = ParameterValue::Float(val);
            }
            ValueRange::ChoiceRange(selection_list) => {
                let val = if let ParameterValue::Choice(x) = *val { x as usize } else { panic!(); };
                if val < selection_list.len() {
                    item.value = ParameterValue::Choice(val);
                }
            }
            ValueRange::NoRange => {}
        };
    }

    fn select_param(item: &mut SelectedItem) {
        // The value in the selected parameter needs to point to the right type
        let val_range = &item.item_list[item.item_index].val_range;
        match val_range {
            ValueRange::IntRange(min, _) => {
                item.value = ParameterValue::Int(*min);
            }
            ValueRange::FloatRange(min, _) => {
                item.value = ParameterValue::Float(*min);
            }
            ValueRange::ChoiceRange(choice_list) => {
                item.value = ParameterValue::Choice(0);
            }
            _ => ()
        }
    }

    fn send_event(&self) {
        let function = &self.selected_function.item_list[self.selected_function.item_index];
        let function_id = if let ParameterValue::Int(x) = &self.selected_function.value { *x as usize } else { panic!() };
        let parameter = &self.selected_parameter.item_list[self.selected_parameter.item_index];
        let param_val = &self.selected_parameter.value;
        let param = SynthParam::new(function.item, function_id, parameter.item, *param_val);
        self.sender.send(SynthMessage::Param(param)).unwrap();
    }

    fn display(&self) {
        let mut x_pos: i32 = 1;
        print!("{}{}", clear::All, cursor::Goto(1, 1));
        self.display_function();
        if self.state == TuiState::FunctionIndex {
            x_pos = 12;
        }
        self.display_function_index();
        if self.state == TuiState::Param || self.state == TuiState::Value {
            if self.state == TuiState::Param {
                x_pos = 14;
            }
            self.display_param();
            if self.state == TuiState::Value {
                x_pos = 23;
            }
            self.display_value();
        }
        //print!("{}", clear::UntilNewline);
        self.display_options(x_pos);
        print!("{}{}", color::Bg(Black), color::Fg(White));
        self.canvas.render(1, 10);
        print!("{}{}", color::Bg(Rgb(255, 255, 255)), color::Fg(Black));
        io::stdout().flush().ok();
    }

    fn display_function(&self) {
        if self.state == TuiState::Function {
            print!("{}{}", color::Bg(LightWhite), color::Fg(Black));
        } else {
            print!("{}{}", color::Bg(Rgb(255, 255, 255)), color::Fg(Black));
        }
        print!("{}", self.selected_function.item_list[self.selected_function.item_index].item);
        if self.state == TuiState::Function {
            print!("{}{}", color::Bg(Rgb(255, 255, 255)), color::Fg(Black));
        }
    }

    fn display_function_index(&self) {
        if self.state == TuiState::FunctionIndex {
            print!("{}{}", color::Bg(LightWhite), color::Fg(Black));
        }
        let function_id = if let ParameterValue::Int(x) = &self.selected_function.value { *x as usize } else { panic!() };
        print!(" {}", function_id);
        if self.state == TuiState::FunctionIndex {
            print!("{}{}", color::Bg(Rgb(255, 255, 255)), color::Fg(Black));
        }
    }

    fn display_param(&self) {
        let item = &self.selected_parameter;
        if self.state == TuiState::Param {
            print!("{}{}", color::Bg(LightWhite), color::Fg(Black));
        }
        print!(" {}", item.item_list[item.item_index].item);
        if self.state == TuiState::Param {
            print!("{}{}", color::Bg(Rgb(255, 255, 255)), color::Fg(Black));
        }
    }

    fn display_value(&self) {
        let item = &self.selected_parameter;
        if self.state == TuiState::Value {
            print!("{}{}", color::Bg(LightWhite), color::Fg(Black));
        }
        match item.value {
            ParameterValue::Int(x) => print!(" {}", x),
            ParameterValue::Float(x) => print!(" {}", x),
            ParameterValue::Choice(x) => {
                let item = &item.item_list[item.item_index];
                let range = &item.val_range;
                let selection = if let ValueRange::ChoiceRange(list) = range { list } else { panic!() };
                let item = selection[x].item;
                print!(" {}", item);
            },
            _ => ()
        }
        if self.state == TuiState::Value {
            print!("{}{}", color::Bg(Rgb(255, 255, 255)), color::Fg(Black));
        }
    }

    fn display_options(&self, x_pos: i32) {
        if self.state == TuiState::Function {
            let mut y_item = 2;
            let list = self.selected_function.item_list;
            for item in list.iter() {
                print!("{}{}", cursor::Goto(x_pos, y_item), item.item);
                y_item += 1;
            }
        }
        if self.state == TuiState::FunctionIndex {
            let item = &self.selected_function.item_list[self.selected_function.item_index];
            let (min, max) = if let ValueRange::IntRange(min, max) = item.val_range { (min, max) } else { panic!() };
            print!("{}{} - {}", cursor::Goto(x_pos, 2), min, max);
        }
        if self.state == TuiState::Param {
            let mut y_item = 2;
            let list = self.selected_parameter.item_list;
            for item in list.iter() {
                print!("{}{}", cursor::Goto(x_pos, y_item), item.item);
                y_item += 1;
            }
        }
        if self.state == TuiState::Value {
            let range = &self.selected_parameter.item_list[self.selected_parameter.item_index].val_range;
            match range {
                ValueRange::IntRange(min, max) => print!("{}{} - {}", cursor::Goto(x_pos, 2), min, max),
                ValueRange::FloatRange(min, max) => print!("{}{} - {}", cursor::Goto(x_pos, 2), min, max),
                ValueRange::ChoiceRange(list) => print!("{}1 - {}", cursor::Goto(x_pos, 2), list.len()),
                ValueRange::NoRange => ()
            }
        }
    }
}
