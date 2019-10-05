use super::parameter::{FunctionId, Parameter, ParameterValue, SynthParam};
use super::TermionWrapper;

use termion::clear;
use termion::event::Key;
use termion::cursor::{DetectCursorPos, Goto};
use std::io::{stdout, stdin};
use std::convert::TryInto;

//use std::sync::mpsc::{Sender, Receiver};
use crossbeam_channel::unbounded;
use crossbeam_channel::{Sender, Receiver};

use std::fmt::{self, Debug, Display};

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

enum ValueRange {
    IntRange(u64, u64),
    FloatRange(f32, f32),
    ChoiceRange(&'static [Selection]),
    NoRange
}

/* Item for a list of selectable functions */
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

static OSC_PARAMS: [Selection; 3] = [
    Selection{item: Parameter::Waveform,  key: Key::Char('w'), val_range: ValueRange::ChoiceRange(&WAVEFORM), next: &[]},
    Selection{item: Parameter::Frequency, key: Key::Char('f'), val_range: ValueRange::FloatRange(0.0, 22000.0), next: &[]},
    Selection{item: Parameter::Phase,     key: Key::Char('p'), val_range: ValueRange::FloatRange(0.0, 100.0), next: &[]},
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
    Selection{item: Parameter::Attack,  key: Key::Char('a'), val_range: ValueRange::FloatRange(0.0, 10.0), next: &[]},
    Selection{item: Parameter::Decay,   key: Key::Char('d'), val_range: ValueRange::FloatRange(0.0, 10.0), next: &[]},
    Selection{item: Parameter::Sustain, key: Key::Char('s'), val_range: ValueRange::FloatRange(0.0, 100.0), next: &[]},
    Selection{item: Parameter::Release, key: Key::Char('r'), val_range: ValueRange::FloatRange(0.0, 10.0), next: &[]},
];

static WAVEFORM: [Selection; 3] = [
    Selection{item: Parameter::Sine,      key: Key::Char('s'), val_range: ValueRange::NoRange, next: &[]},
    Selection{item: Parameter::Square,    key: Key::Char('q'), val_range: ValueRange::NoRange, next: &[]},
    Selection{item: Parameter::Triangle,  key: Key::Char('t'), val_range: ValueRange::NoRange, next: &[]},
];

pub struct Tui {
    // Function selection
    state: TuiState,
    sender: Sender<SynthParam>,
    receiver: Receiver<SynthParam>,

    // TUI handling
    current_list: &'static [Selection],
    selected_function: SelectedItem,
    selected_parameter: SelectedItem,

    temp_string: String,
}

struct SelectedItem {
    item_list: &'static [Selection], // The selection this item is coming from
    item_index: usize, // Index into the selection list
    value: ParameterValue, // ID or value of the selected item
    x: u16, // Cursor position of the item
    y: u16,
    val_x: u16, // Cursor position of the item value
    val_y: u16,
}

impl Tui {
    pub fn new(sender: Sender<SynthParam>, receiver: Receiver<SynthParam>) -> Tui {
        //let (x, y) = stdout().cursor_pos().unwrap();; //self.termion.cursor_pos().unwrap();
        let (x, y) = (1u16, 1u16);
        let state = TuiState::Init;
        let current_list = &FUNCTIONS;
        let selected_function = SelectedItem{item_list: &FUNCTIONS, item_index: 0, value: ParameterValue::Int(0), x: x, y: y, val_x: x, val_y: y};
        let selected_parameter = SelectedItem{item_list: &OSC_PARAMS, item_index: 0, value: ParameterValue::Int(0), x: x, y: y, val_x: x, val_y: y};
        let temp_string = String::new();
        Tui{state,
            sender,
            receiver,
            current_list,
            selected_function,
            selected_parameter,
            temp_string
        }
    }

    pub fn handle_input(&mut self, c: termion::event::Key) {
        let (x, y) = Tui::push_cursor(1, 5);
        print!("handle_input in {}{}", self.state, clear::UntilNewline);
        Tui::restore_cursor(x, y);

        let new_state = match self.state {
            TuiState::Init => {
                self.init();
                TuiState::Function
            }
            TuiState::Function => self.get_function(c),
            TuiState::FunctionIndex => self.get_function_index(c),
            TuiState::Param => self.get_param(c),
            TuiState::Value => self.get_value(c),
            TuiState::EventComplete => {
                self.send_event();
                TuiState::Function
            }
        };
        self.state = new_state;
    }

    fn restore_cursor(x: u16, y: u16) {
        print!("{}", Goto(x, y));
    }

    fn push_cursor(x: u16, y: u16) -> (u16, u16) {
        let (cur_x, cur_y) = stdout().cursor_pos().unwrap();
        Tui::restore_cursor(x, y);
        (cur_x, cur_y)
    }

    fn init(&mut self) {
        print!("{}{}", clear::All, Goto(1, 1));
        let (x, y) = stdout().cursor_pos().unwrap(); //self.termion.cursor_pos().unwrap();
        self.selected_function.x = x;
        self.selected_function.y = y;
    }

    fn send_event(&self) {
        let function = &self.selected_function.item_list[self.selected_function.item_index];
        let function_id = if let ParameterValue::Int(x) = &self.selected_function.value { x } else { panic!() };
        let parameter = &self.selected_parameter.item_list[self.selected_parameter.item_index];
        let param_val = &self.selected_function.value;
        self.sender.send(SynthParam::new(function.item, FunctionId::Int(*function_id), parameter.item, *param_val)).unwrap();
    }

    fn get_function(&mut self, c: termion::event::Key) -> TuiState {
        match c {
            Key::Up        => {
                if self.selected_function.item_index < self.selected_function.item_list.len() - 1 {
                    self.selected_function.item_index += 1;
                }
                self.select_function();
                self.select_function_index();
                TuiState::Function
            }
            Key::Down      => {
                if self.selected_function.item_index > 0 {
                    self.selected_function.item_index -= 1;
                }
                self.select_function();
                self.select_function_index();
                TuiState::Function
            }
            Key::Right     => {
                self.select_function_index();
                TuiState::FunctionIndex
            }
            _ => {
                let state = self.handle_key_selection(c);
                self.select_function();
                self.select_function_index();
                state
            }
        }
    }

    fn handle_key_selection(&mut self, c: termion::event::Key) -> TuiState {
        for (count, f) in self.selected_function.item_list.iter().enumerate() {
            if f.key == c {
                self.selected_function.item_index = count;
                self.select_function();
                return TuiState::FunctionIndex
            } else {
            }
        }
        TuiState::Function
    }

    fn select_function(&mut self) {
        print!("{}{}", Goto(self.selected_function.x, self.selected_function.y), clear::UntilNewline);
        print!("{}", self.selected_function.item_list[self.selected_function.item_index].item);
        let (x, y) = stdout().cursor_pos().unwrap();; //self.termion.cursor_pos().unwrap();
        self.selected_function.val_x = x;
        self.selected_function.val_y = y;
        self.selected_parameter.item_list = self.selected_function.item_list[self.selected_function.item_index].next;
    }

    fn get_function_index(&mut self, c: termion::event::Key) -> TuiState {
        let val = self.selected_function.value;
        let val_range = &self.selected_function.item_list[self.selected_function.item_index].val_range;
        let (min, max) = if let ValueRange::IntRange(i, j) = val_range { (i, j) } else { panic!() };
        let mut current = if let ParameterValue::Int(x) = val { x } else { panic!() };
        match c {
            Key::Char(x) => {
                match x {
                    '0' | '1' | '2' | '3' | '4' | '5' | '6' | '7' | '8' | '9' => {
                        let y = x as u64 - '0' as u64;
                        let val_digit_added = current * 10 + y;
                        if val_digit_added > *max {
                            // Can't add another digit, replace current value with new one
                            current = y;
                        } else {
                            current = val_digit_added;
                        }
                        let new_state = if y * 10 > *max {
                            // Can't add another digit, accept value as final and move on
                            self.select_function_index();
                            self.select_param();
                            TuiState::Param
                        } else {
                            // Could add more digits, not finished yet
                            TuiState::FunctionIndex
                        };
                        self.update_index_val(current, new_state)
                    }
                    _ => { self.select_function_index(); self.select_param(); TuiState::Param},
                }
            }
            Key::Up        => self.update_index_val(current + 1, TuiState::FunctionIndex),
            Key::Down      => self.update_index_val(if current > 0 { current - 1 } else { current }, TuiState::FunctionIndex),
            Key::Right     => { self.select_function_index(); self.select_param(); TuiState::Param},
            _ => {self.select_function(); TuiState::Function}
        }
    }

    fn update_index_val(&mut self, val: u64, s: TuiState) -> TuiState {
        match self.selected_function.item_list[self.selected_function.item_index].val_range {
            ValueRange::IntRange(min, max) => {
                if val <= max && val >= min {
                    self.selected_function.value = ParameterValue::Int(val.try_into().unwrap());
                }
            }
            ValueRange::FloatRange(min, max) => {}
            ValueRange::ChoiceRange(_) => {}
            ValueRange::NoRange => {}
        };
        self.select_function_index();
        s
    }

    fn select_function_index(&mut self) {
        print!("{} {:?}", Goto(self.selected_function.val_x, self.selected_function.val_y), self.selected_function.value);
        let (x, y) = stdout().cursor_pos().unwrap();; //self.termion.cursor_pos().unwrap();
        self.selected_parameter.x = x;
        self.selected_parameter.y = y;
        self.selected_parameter.item_list = self.selected_function.item_list[self.selected_function.item_index].next;
    }

    fn get_param(&mut self, c: termion::event::Key) -> TuiState {
        let next_state = match c {
            Key::Up => {
                if self.selected_parameter.item_index < self.selected_parameter.item_list.len() - 1 {
                    self.selected_parameter.item_index += 1;
                }
                TuiState::Param
            }
            Key::Down => {
                if self.selected_parameter.item_index > 0 {
                    self.selected_parameter.item_index -= 1;
                }
                TuiState::Param
            }
            Key::Right => {
                TuiState::Value
            }
            Key::Left => TuiState::FunctionIndex,
            _ => {
                self.handle_param_key_selection(c)
            }
        };
        if next_state == TuiState::Param {
            self.select_param();
        }
        next_state
    }

    fn handle_param_key_selection(&mut self, c: termion::event::Key) -> TuiState {
        for (count, f) in self.selected_parameter.item_list.iter().enumerate() {
            if f.key == c {
                self.selected_parameter.item_index = count;
                self.select_param();
                return TuiState::Value
            } else {
            }
        }
        TuiState::Function
    }

    fn select_param(&mut self) {
        // The value in the selected parameter needs to point to the right type
        let item = &self.selected_parameter.item_list[self.selected_parameter.item_index].item;
        let val_range = &self.selected_parameter.item_list[self.selected_parameter.item_index].val_range;
        match val_range {
            ValueRange::IntRange(min, _) => {
                self.selected_parameter.value = ParameterValue::Int(*min);
            }
            ValueRange::FloatRange(min, _) => {
                self.selected_parameter.value = ParameterValue::Float(*min);
            }
            ValueRange::ChoiceRange(choice_list) => {
                self.selected_parameter.value = ParameterValue::Choice(0);
            }
            _ => ()
        }
        Tui::display_param(&mut self.selected_parameter);
        Tui::display_value(&self.selected_parameter);
    }

    fn get_value(&mut self, c: termion::event::Key) -> TuiState {
        let val = self.selected_parameter.value;
        match self.selected_parameter.item_list[self.selected_parameter.item_index].val_range {
            ValueRange::IntRange(min, max) => {
                let mut current = if let ParameterValue::Int(x) = val { x } else { panic!() };
                match c {
                    Key::Char(x) => {
                        match x {
                            '0' | '1' | '2' | '3' | '4' | '5' | '6' | '7' | '8' | '9' => {
                                let y = x as u64 - '0' as u64;
                                let val_digit_added = current * 10 + y;
                                if val_digit_added > max {
                                    // Can't add another digit, replace current value with new one
                                    current = y;
                                } else {
                                    current = val_digit_added;
                                }
                                let new_state = if y * 10 > max {
                                    // Can't add another digit, accept value as final and move on
                                    TuiState::Param
                                } else {
                                    // Could add more digits, not finished yet
                                    TuiState::Value
                                };
                                self.update_param_value(current, new_state)
                            }
                            '\n' => TuiState::EventComplete,
                            _ => TuiState::Value,
                        }
                    }
                    Key::Up        => self.update_param_value(current + 1, TuiState::Value),
                    Key::Down      => self.update_param_value(if current > 0 { current - 1 } else { current }, TuiState::Value),
                    Key::Left => {
                        self.select_param();
                        TuiState::Param
                    }
                    _ => {self.select_function(); TuiState::Param}
                }
            }
            ValueRange::FloatRange(min, max) => {
                let current = if let ParameterValue::Float(x) = val { x } else { panic!() };
                match c {
                    Key::Char(x) => {
                        match x {
                            '0' | '1' | '2' | '3' | '4' | '5' | '6' | '7' | '8' | '9' | '.' => {
                                self.temp_string.push(x);
                                self.update_param_value(0, TuiState::Value)
                            }
                            '\n' => TuiState::EventComplete,
                            _ => TuiState::FunctionIndex,
                        }
                    }
                    Key::Left => {
                        self.select_param();
                        TuiState::Param
                    }
                    Key::Backspace => {
                        self.temp_string.pop();
                        self.update_param_value(0, TuiState::Value)
                    }
                    _ => {self.select_function(); TuiState::Param}
                }
            }
            ValueRange::ChoiceRange(choice_list) => {
                let current = if let ParameterValue::Choice(x) = val { x } else { panic!() };
                match c {
                    Key::Up        => self.update_param_value(current as u64 + 1, TuiState::Value),
                    Key::Down      => self.update_param_value(if current > 0 { current as u64 - 1 } else { current as u64 }, TuiState::Value),
                    Key::Left => {
                        self.select_param();
                        TuiState::Param
                    }
                    Key::Char('\n') => TuiState::EventComplete,
                    _ => TuiState::Value
                }
            }
            _ => TuiState::Value
        }
    }

    fn update_param_value(&mut self, val: u64, s: TuiState) -> TuiState {
        match self.selected_parameter.item_list[self.selected_parameter.item_index].val_range {
            ValueRange::IntRange(min, max) => {
                if val <= max && val >= min {
                    self.selected_parameter.value = ParameterValue::Int(val.try_into().unwrap());
                }
            }
            ValueRange::FloatRange(min, max) => {
                let mut value: f32 = self.temp_string.parse().unwrap();
                if value > max {
                    value = max;
                    self.temp_string = value.to_string();
                }
                if value < min {
                    value = min;
                    self.temp_string = value.to_string();
                }
                self.selected_parameter.value = ParameterValue::Float(value);
            }
            ValueRange::ChoiceRange(selection_list) => {
                let val = val as usize;
                if val < selection_list.len() {
                    self.selected_parameter.value = ParameterValue::Choice(val);
                }
            }
            ValueRange::NoRange => {}
        };
        Tui::display_value(&self.selected_parameter);
        s
    }

    fn display_param(item: &mut SelectedItem) {
        let param = item.item_list[item.item_index].item;
        let val_range = &item.item_list[item.item_index].val_range;
        print!("{}{}", Goto(item.x, item.y), clear::UntilNewline);
        print!("{}", param);
        let (x, y) = stdout().cursor_pos().unwrap();; //self.termion.cursor_pos().unwrap();
        item.val_x = x;
        item.val_y = y;
    }

    fn display_value(item: &SelectedItem) {
        print!("{} ", Goto(item.val_x, item.val_y));
        match item.value {
            ParameterValue::Int(x) => print!("{}", x),
            ParameterValue::Float(x) => print!("{}", x),
            ParameterValue::Choice(x) => {
                let item = &item.item_list[item.item_index];
                let range = &item.val_range;
                let selection = if let ValueRange::ChoiceRange(list) = range { list } else { panic!() };
                let item = selection[x].item;
                print!("{}{}", item, clear::UntilNewline);
            },
            _ => ()
        }
    }
}
