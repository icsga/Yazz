use super::parameter::{Function, FunctionId, Parameter, ParameterValue, SynthParam};
use super::TermionWrapper;

use termion::clear;
use termion::event::Key;
use termion::cursor::{DetectCursorPos, Goto};

use std::sync::mpsc::{Sender, Receiver};

enum TuiState {
    Function,
    FunctionIndex,
    Param,
    ParamIndex,
    Value,
    EventComplete
}

/*
enum ValueRange {
    IntRange(u32, u32),
    FloatRange(f32, f32),
    ChoiceRange
    NoRange
}

/* Item for a list of selectable functions */
struct Selection {
    function: Function,
    key: Key,
    val_range: ValueRange,
    next: &'static [Selection]
}

static FUNCTIONS: [Selection; 2] = [
    Selection{id: FunctionId::Oscillator, key: Key::Char('o'), val_type: ValType::Int, int_min: 1, int_max: 3, f_min: 0.0, f_max: 0.0, next: &OSC_PARAMS},
    Selection{id: FunctionId::Lfo,        key: Key::Char('l'), val_type: ValType::Int, int_min: 1, int_max: 3, f_min: 0.0, f_max: 0.0, next: &[]},
];

static OSC_PARAMS: [Selection; 3] = [
    Selection{id: FunctionId::Waveform, key: Key::Char('w'), val_type: ValType::Int, int_min: 1, int_max: 3, f_min: 0.0, f_max: 0.0, next: &[]},
    Selection{id: FunctionId::Frequency, key: Key::Char('f'), val_type: ValType::Float, int_min: 0, int_max: 0, f_min: 0.0, f_max: 22000.0, next: &[]},
    Selection{id: FunctionId::Phase, key: Key::Char('p'), val_type: ValType::Float, int_min: 0, int_max: 0, f_min: 0.0, f_max: 100.0, next: &[]},
];

fn select_function(funcs: &[Selection], next_state: TuiState) {
}
*/

pub struct Tui {
    // Function selection
    state: TuiState,
    selected_function: Function,
    function_index: u32,
    selected_parameter: Parameter,
    parameter_index: u32,
    sender: Sender<SynthParam>,
    receiver: Receiver<SynthParam>,

    // TUI handling
    x: u16,
    y: u16,
}

impl Tui {
    pub fn new(sender: Sender<SynthParam>, receiver: Receiver<SynthParam>) -> Tui {
        Tui{state: TuiState::Function,
              selected_function: Function::Oscillator,
              function_index: 1,
              selected_parameter: Parameter::Waveform,
              parameter_index: 1,
              sender: sender,
              receiver: receiver,
              x: 0,
              y: 0,}
    }

    pub fn handle_input(&mut self, c: termion::event::Key) {
        // Test: Any key triggers the envelope
        let param = SynthParam::new();
        self.sender.send(param).unwrap();
        return;
        /*
        self.state = match &self.state {
            TuiState::Function => self.get_function(c),
            TuiState::FunctionIndex => self.get_function_index(c),
            TuiState::Param => self.get_param(c),
            TuiState::ParamIndex => self.get_param_index(c),
            TuiState::Value => self.get_value(c),
            TuiState::EventComplete => TuiState::Function
        };
        */
    }

    fn select_func(&mut self, f: Function, s: TuiState) -> TuiState {
        self.selected_function = f;
        self.function_index = 0;
        print!("{}\r", clear::CurrentLine);
        print!("Function: {}", self.selected_function);
        //let (x, y) = self.stdout.cursor_pos().unwrap();
        //self.x = x;
        //self.y = y;
        s
    }

    fn get_function(&mut self, c: termion::event::Key) -> TuiState {
        match c {
            Key::Char('o') => self.select_func(Function::Oscillator, TuiState::FunctionIndex),
            Key::Char('l') => self.select_func(Function::Lfo, TuiState::FunctionIndex),
            _ => TuiState::Function // Ignore others
        }
    }

    fn select_func_index(&mut self, val: u32, s: TuiState) -> TuiState {
        self.function_index = val;
        print!("{} {}", Goto(self.x, self.y), self.function_index);
        s
    }

    fn get_function_index(&mut self, c: termion::event::Key) -> TuiState {
        let val: u32 = self.function_index;
        match c {
            Key::Char('0') => self.select_func_index(val * 10 + 0, TuiState::Param),
            Key::Char('1') => self.select_func_index(val * 10 + 1, TuiState::Param),
            Key::Char('2') => self.select_func_index(val * 10 + 2, TuiState::Param),
            Key::Char('3') => self.select_func_index(val * 10 + 3, TuiState::Param),
            Key::Char('4') => self.select_func_index(val * 10 + 4, TuiState::Param),
            Key::Char('5') => self.select_func_index(val * 10 + 5, TuiState::Param),
            Key::Char('6') => self.select_func_index(val * 10 + 6, TuiState::Param),
            Key::Char('7') => self.select_func_index(val * 10 + 7, TuiState::Param),
            Key::Char('8') => self.select_func_index(val * 10 + 8, TuiState::Param),
            Key::Char('9') => self.select_func_index(val * 10 + 9, TuiState::Param),
            Key::Up        => self.select_func_index(val + 1, TuiState::FunctionIndex),
            Key::Down      => self.select_func_index(if val > 0 { val - 1 } else { val }, TuiState::FunctionIndex),
            _ => return TuiState::Function
        }
    }

    fn get_param(&mut self, c: termion::event::Key) -> TuiState {
        let val: u32 = self.function_index;
        match c {
            Key::Up        => self.select_func_index(val + 1, TuiState::FunctionIndex),
            Key::Down      => self.select_func_index(if val > 1 { val - 1 } else { val }, TuiState::FunctionIndex),
            _ => TuiState::Function
        }
    }

    fn get_param_index(&mut self, c: termion::event::Key) -> TuiState {
        TuiState::Function
    }

    fn get_value(&mut self, c: termion::event::Key) -> TuiState {
        TuiState::Function
    }
}
