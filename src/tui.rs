extern crate termion;

use termion::clear;
use termion::cursor;
use termion::cursor::{DetectCursorPos, Goto};
use termion::event::Key;
use termion::input::TermRead;
use termion::raw::{IntoRawMode, RawTerminal};

use std::io::{Write, stdout, stdin};

enum TuiState {
    Function,
    FunctionIndex,
    Param,
    ParamIndex,
    Value,
    EventComplete
}

#[derive(Debug)]
enum FunctionId {
    Oscillator,
    Filter,
    Amp,
    Lfo,
    Envelope,
    Mod,

    // Oscillator, Lfo
    Waveform,
    Frequency,
    Phase,

    // Filter
    Type,
    FilterFreq,
    Resonance,

    // Amp
    Volume,

    // Lfo

    // Envelope
    Attack,
    Decay,
    Sustain,
    Release,

    // Mod
    Source,
    Target
}

enum Parameter {
}

enum ValType {
    Int,
    Float,
    NoValue
}

use std::fmt::{self, Debug, Display};

impl fmt::Display for FunctionId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

/* Item for a list of selectable functions */
struct Selection {
    id: FunctionId,
    key: Key,
    val_type: ValType,
    int_min: u32,
    int_max: u32,
    f_min: f32,
    f_max: f32,
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

pub struct Tui {
    // Function selection
    state: TuiState,
    selected_function: FunctionId,
    function_index: u32,
    selected_parameter: FunctionId,
    parameter_index: u32,

    // TUI handling
    x: u16,
    y: u16,
    stdout: RawTerminal<std::io::Stdout>,
}

impl Tui {
    pub fn new() -> Self {
        Tui{state: TuiState::Function,
            selected_function: FunctionId::Oscillator,
            function_index: 1,
            selected_parameter: FunctionId::Waveform,
            parameter_index: 1,
            x: 0,
            y: 0,
            stdout: stdout().into_raw_mode().unwrap()}
    }

    pub fn handle_input(&mut self) {
        let mut exit = false;

        loop {
            let stdin = stdin();
            for c in stdin.keys() {
                let c = c.unwrap();
                match c {
                    // Exit.
                    Key::Char('q') => { exit = true; break},
                    _              => self.select_item(c),
                }
                self.stdout.flush().unwrap();
            }
            if exit {
                println!("");
                return;
            }
        }
    }

    fn select_item(&mut self, c: termion::event::Key) {
        self.state = match &self.state {
            TuiState::Function => self.get_function(c),
            TuiState::FunctionIndex => self.get_function_index(c),
            TuiState::Param => self.get_param(c),
            TuiState::ParamIndex => self.get_param_index(c),
            TuiState::Value => self.get_value(c),
            TuiState::EventComplete => TuiState::Function
        };
    }

    fn select_func(&mut self, f: FunctionId, s: TuiState) -> TuiState {
        self.selected_function = f;
        self.function_index = 0;
        print!("{}\r", clear::CurrentLine);
        print!("Function: {}", self.selected_function);
        let (x, y) = self.stdout.cursor_pos().unwrap();
        self.x = x;
        self.y = y;
        s
    }

    fn get_function(&mut self, c: termion::event::Key) -> TuiState {
        match c {
            Key::Char('o') => self.select_func(FunctionId::Oscillator, TuiState::FunctionIndex),
            Key::Char('l') => self.select_func(FunctionId::Lfo, TuiState::FunctionIndex),
            _ => TuiState::Function // Ignore others
        }
    }

    fn select_func_index(&mut self, val: u32, s: TuiState) -> TuiState {
        self.function_index = val;
        print!("{} {}", Goto(self.x, self.y), self.function_index);
        s
    }

    fn get_function_index(&mut self, c: termion::event::Key) -> TuiState {
        let mut val: u32 = self.function_index;
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
        let mut val: u32 = self.function_index;
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
