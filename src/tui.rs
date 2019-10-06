use super::parameter::{FunctionId, Parameter, ParameterValue, SynthParam};
use super::TermionWrapper;

use termion::clear;
use termion::event::Key;
use termion::cursor::{DetectCursorPos, Goto};
use termion::color;
use termion::color::{Black, White, LightWhite, Reset, Rgb};
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
                //self.send_event();
                self.init();
                TuiState::Function
            }
        };
        self.state = new_state;
        self.display();
    }

    fn init(&mut self) {
        print!("{}{}", clear::All, Goto(1, 1));
        let (x, y) = stdout().cursor_pos().unwrap(); //self.termion.cursor_pos().unwrap();
        self.selected_function.x = x;
        self.selected_function.y = y;
        self.temp_string.clear();
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
                TuiState::Function
            }
            Key::Down      => {
                if self.selected_function.item_index > 0 {
                    self.selected_function.item_index -= 1;
                }
                TuiState::Function
            }
            Key::Right     => {
                TuiState::FunctionIndex
            }
            _ => {
                let state = self.handle_key_selection(c);
                state
            }
        }
    }

    fn handle_key_selection(&mut self, c: termion::event::Key) -> TuiState {
        for (count, f) in self.selected_function.item_list.iter().enumerate() {
            if f.key == c {
                self.selected_function.item_index = count;
                return TuiState::FunctionIndex
            } else {
            }
        }
        TuiState::Function
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
                            self.select_param();
                            TuiState::Param
                        } else {
                            // Could add more digits, not finished yet
                            TuiState::FunctionIndex
                        };
                        self.update_index_val(current, new_state)
                    }
                    _ => {
                        self.select_param();
                        TuiState::Param
                    },
                }
            }
            Key::Up        => self.update_index_val(current + 1, TuiState::FunctionIndex),
            Key::Down      => self.update_index_val(if current > 0 { current - 1 } else { current }, TuiState::FunctionIndex),
            Key::Right     => {
                self.select_param();
                TuiState::Param
            },
            _ => {
                TuiState::Function
            }
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
        s
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
    }

    fn get_value(&mut self, c: termion::event::Key) -> TuiState {
        let val = self.selected_parameter.value;
        let mut new_state = TuiState::Value;
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
                                new_state = if y * 10 > max {
                                    // Can't add another digit, accept value as final and move on
                                    TuiState::Param
                                } else {
                                    // Could add more digits, not finished yet
                                    TuiState::Value
                                };
                            },
                            '\n' => new_state = TuiState::EventComplete,
                            _ => new_state = TuiState::Value,
                        }
                    }
                    Key::Up        => current += 1,
                    Key::Down      => if current > 0 { current -= 1 },
                    Key::Left => {
                        self.select_param();
                        new_state = TuiState::Param
                    }
                    _ => {
                        new_state = TuiState::Param
                    }
                }
                self.update_param_value(ParameterValue::Int(current), new_state)
            }
            ValueRange::FloatRange(min, max) => {
                let mut current = if let ParameterValue::Float(x) = val { x } else { panic!() };
                match c {
                    Key::Char(x) => {
                        match x {
                            '0' | '1' | '2' | '3' | '4' | '5' | '6' | '7' | '8' | '9' | '.' => {
                                self.temp_string.push(x);
                                let value = self.temp_string.parse();
                                current = if let Ok(x) = value { x } else { current };
                            },
                            '\n' => new_state = TuiState::EventComplete,
                            _ => new_state = TuiState::FunctionIndex,
                        }
                    }
                    Key::Up        => current += 1.0,
                    Key::Down      => current -= 1.0,
                    Key::Left => {
                        self.select_param();
                        new_state = TuiState::Param
                    }
                    Key::Backspace => {
                        let len = self.temp_string.len();
                        if len > 0 {
                            self.temp_string.pop();
                            if len > 1 {
                                let value = self.temp_string.parse();
                                current = if let Ok(x) = value { x } else { current };
                            } else {
                                new_state = TuiState::Param
                            }
                        } else {
                            new_state = TuiState::Param
                        }
                    }
                    _ => new_state = TuiState::Param
                }
                self.update_param_value(ParameterValue::Float(current), new_state)
            }
            ValueRange::ChoiceRange(choice_list) => {
                let mut current = if let ParameterValue::Choice(x) = val { x } else { panic!() };
                match c {
                    Key::Up        => current += 1,
                    Key::Down      => if current > 0 { current -= 1 },
                    Key::Left => {
                        self.select_param();
                        new_state = TuiState::Param
                    }
                    Key::Char('\n') => new_state = TuiState::EventComplete,
                    _ => new_state = TuiState::Value
                }
                self.update_param_value(ParameterValue::Choice(current), new_state)
            }
            _ => TuiState::Value
        }
    }

    fn update_param_value(&mut self, val: ParameterValue, s: TuiState) -> TuiState {
        match self.selected_parameter.item_list[self.selected_parameter.item_index].val_range {
            ValueRange::IntRange(min, max) => {
                let val = if let ParameterValue::Int(x) = val { x } else { panic!(); };
                if val <= max && val >= min {
                    self.selected_parameter.value = ParameterValue::Int(val.try_into().unwrap());
                }
            }
            ValueRange::FloatRange(min, max) => {
                let mut val = if let ParameterValue::Float(x) = val { x } else { panic!(); };
                if val > max {
                    val = max;
                    self.temp_string = val.to_string();
                }
                if val < min {
                    val = min;
                    self.temp_string = val.to_string();
                }
                self.selected_parameter.value = ParameterValue::Float(val);
            }
            ValueRange::ChoiceRange(selection_list) => {
                let val = if let ParameterValue::Float(x) = val { x as usize } else { panic!(); };
                if val < selection_list.len() {
                    self.selected_parameter.value = ParameterValue::Choice(val);
                }
            }
            ValueRange::NoRange => {}
        };
        self.send_event();
        s
    }

    fn display(&mut self) {
        print!("{}", Goto(self.selected_function.x, self.selected_function.y));
        self.display_function();
        self.display_function_index();
        if self.state == TuiState::Function || self.state == TuiState::FunctionIndex {
            print!("{}", clear::UntilNewline);
            return;
        }
        self.display_param();
        self.display_value();
        print!("{}", clear::UntilNewline);
    }

    fn display_function(&mut self) {
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

    fn display_function_index(&mut self) {
        if self.state == TuiState::FunctionIndex {
            print!("{}{}", color::Bg(LightWhite), color::Fg(Black));
        }
        print!(" {:?}", self.selected_function.value);
        if self.state == TuiState::FunctionIndex {
            print!("{}{}", color::Bg(Rgb(255, 255, 255)), color::Fg(Black));
        }
        self.selected_parameter.item_list = self.selected_function.item_list[self.selected_function.item_index].next;
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
}
