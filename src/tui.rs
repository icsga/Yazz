use super::parameter::{Parameter, ParameterValue, SynthParam};
use super::midi_handler::{MidiMessage, MessageType};
use super::{UiMessage, SynthMessage};
use super::canvas::Canvas;

use crossbeam_channel::{Sender, Receiver};
use termion::{clear, color, cursor};
use termion::color::{Black, White, LightWhite, Reset, Rgb};
use termion::event::Key;

use log::{info, trace, warn};

use std::convert::TryInto;
use std::fmt::{self, Debug};
use std::io;
use std::io::{stdout, Write};
use std::num::ParseFloatError;
use std::thread::spawn;
use std::time::{Duration, SystemTime};

#[derive(Copy, Clone, PartialEq, Debug)]
enum TuiState {
    Function,
    FunctionIndex,
    Param,
    Value,
}

impl fmt::Display for TuiState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

fn next(current: TuiState) -> TuiState {
    use TuiState::*;
    match current {
        Function => FunctionIndex,
        FunctionIndex => Param,
        Param => Value,
        Value => Param,
    }
}

fn previous(current: TuiState) -> TuiState {
    use TuiState::*;
    match current {
        Function => Function,
        FunctionIndex => Function,
        Param => FunctionIndex,
        Value => Param,
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

/* Top-level menu */
static FUNCTIONS: [Selection; 4] = [
    Selection{item: Parameter::Oscillator, key: Key::Char('o'), val_range: ValueRange::IntRange(1, 3), next: &OSC_PARAMS},
    Selection{item: Parameter::Envelope,   key: Key::Char('e'), val_range: ValueRange::IntRange(1, 2), next: &ENV_PARAMS},
    Selection{item: Parameter::Lfo,        key: Key::Char('l'), val_range: ValueRange::IntRange(1, 3), next: &LFO_PARAMS},
    Selection{item: Parameter::Filter,     key: Key::Char('f'), val_range: ValueRange::IntRange(1, 2), next: &FILTER_PARAMS},
];

static OSC_PARAMS: [Selection; 9] = [
    Selection{item: Parameter::Waveform,  key: Key::Char('w'), val_range: ValueRange::ChoiceRange(&WAVEFORM), next: &[]},
    Selection{item: Parameter::Level,     key: Key::Char('l'), val_range: ValueRange::FloatRange(0.0, 100.0), next: &[]},
    Selection{item: Parameter::Frequency, key: Key::Char('f'), val_range: ValueRange::IntRange(-24, 24), next: &[]},
    Selection{item: Parameter::Blend,     key: Key::Char('b'), val_range: ValueRange::FloatRange(0.0, 5.0), next: &[]},
    Selection{item: Parameter::Phase,     key: Key::Char('p'), val_range: ValueRange::FloatRange(0.0, 1.0), next: &[]},
    Selection{item: Parameter::Sync,      key: Key::Char('s'), val_range: ValueRange::IntRange(0, 1), next: &[]},
    Selection{item: Parameter::KeyFollow, key: Key::Char('k'), val_range: ValueRange::IntRange(0, 1), next: &[]},
    Selection{item: Parameter::Voices,    key: Key::Char('v'), val_range: ValueRange::IntRange(1, 7), next: &[]},
    Selection{item: Parameter::Spread,    key: Key::Char('e'), val_range: ValueRange::FloatRange(0.0, 2.0), next: &[]},
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

#[derive(Debug)]
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

    sync_counter: u32,
    idle: Duration, // Accumulated idle times of the engine
    busy: Duration, // Accumulated busy times of the engine
    min_idle: Duration,
    max_busy: Duration,
    canvas: Canvas,

    temp_string: String,
}

impl Tui {
    pub fn new(sender: Sender<SynthMessage>, ui_receiver: Receiver<UiMessage>) -> Tui {
        let state = TuiState::Function;
        let current_list = &FUNCTIONS;
        let selected_function = SelectedItem{item_list: &FUNCTIONS, item_index: 0, value: ParameterValue::Int(1)};
        let selected_parameter = SelectedItem{item_list: &OSC_PARAMS, item_index: 0, value: ParameterValue::Int(1)};
        let temp_string = String::new();
        let sync_counter = 0;
        let idle = Duration::new(0, 0);
        let busy = Duration::new(0, 0);
        let min_idle = Duration::new(10, 0);
        let max_busy = Duration::new(0, 0);
        let canvas = Canvas::new(100, 30);
        Tui{state,
            sender,
            ui_receiver,
            current_list,
            selected_function,
            selected_parameter,
            sync_counter,
            idle,
            busy,
            min_idle,
            max_busy,
            canvas,
            temp_string
        }
    }

    pub fn run(mut tui: Tui) -> std::thread::JoinHandle<()> {
        let mut get_wave = false;
        let handler = spawn(move || {
            loop {
                //get_wave = true;
                let msg = tui.ui_receiver.recv().unwrap();
                match msg {
                    UiMessage::Midi(m)  => tui.handle_midi(m),
                    UiMessage::Key(m) => tui.handle_input(m),
                    UiMessage::Param(m) => tui.handle_param(m),
                    UiMessage::WaveBuffer(m) => {
                        tui.handle_wavebuffer(m);
                        get_wave = false;
                    },
                    UiMessage::EngineSync(idle, busy) => tui.handle_engine_sync(idle, busy),
                };
                if get_wave {
                    tui.get_waveform();
                }
            }
        });
        handler
    }

    /** MIDI message received */
    pub fn handle_midi(&mut self, m: MidiMessage) {
        match m.get_message_type() {
            MessageType::ControlChg => {
                if m.param == 0x01 { // ModWheel
                    self.handle_control_change(m.value as i64);
                }
            },
            _ => ()
        }
    }

    /** Evaluate the MIDI control change message (ModWheel) */
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

    /** Received a queried parameter value from the synth engine. */
    pub fn handle_param(&mut self, m: SynthParam) {
        info!("handle_param {} = {:?}", self.selected_parameter.item_list[self.selected_parameter.item_index].item, m);
        let item = &mut self.selected_parameter;
        Tui::update_value(item, &m.value, &mut self.temp_string);
    }

    /** Received a buffer with samples from the synth engine. */
    pub fn handle_wavebuffer(&mut self, m: Vec<f32>) {
        self.canvas.clear();
        for (x_pos, v) in m.iter().enumerate() {
            let y_pos = ((v + 1.0) * (29.0 / 2.0)) as usize;
            self.canvas.set(x_pos, y_pos, '∘');
        }
    }

    /** Received a sync signal from the audio engine.
     *
     * This is used to control timing related actions like drawing the display.
     * The message contains timing data of the audio processing loop.
     */
    fn handle_engine_sync(&mut self, idle: Duration, busy: Duration) {
        self.idle += idle;
        self.busy += busy;
        if idle < self.min_idle {
            self.min_idle = idle;
        }
        if busy > self.max_busy {
            self.max_busy = busy;
        }
        self.sync_counter += 1;
        if self.sync_counter == 10 {
            let display_time = SystemTime::now();
            self.display();
            /*
            println!("{}Display: {:?}\r\nIdle: {:?}\r\nBusy: {:?}\r\nLast: {:?}, {:?}\r\nMin Idle: {:?}, Max Busy: {:?}",
                     cursor::Goto(1, 5), display_time.elapsed().unwrap(), self.idle / 10, self.busy / 10, idle, busy, self.min_idle, self.max_busy);
            self.idle = Duration::new(0, 0);
            self.busy = Duration::new(0, 0);
            */
            self.sync_counter = 0;
        }
    }

    /** Received a keyboard event from the terminal. */
    pub fn handle_input(&mut self, c: termion::event::Key) {
        info!("handle_input {:?}", c);
        let new_state = match self.state {
            TuiState::Function => self.select_item(c),
            TuiState::FunctionIndex => self.get_value(c),
            TuiState::Param => self.select_item(c),
            TuiState::Value => self.get_value(c),
        };
        self.change_state(new_state);
    }

    /** Change the state of the input state machine. */
    fn change_state(&mut self, new_state: TuiState) {
        info!("change_state {} -> {}", self.state, new_state);
        if self.state == TuiState::Value && new_state == TuiState::Value{
            self.send_event();
        }
        if new_state != self.state {
            self.state = new_state;
            match new_state {
                TuiState::Function => {
                    // We are probably selecting a different function than
                    // before, so we should start the parameter list at the
                    // beginning to avoid out-of-bound errors.
                    self.selected_parameter.item_index = 0;
                }
                TuiState::FunctionIndex => {}
                TuiState::Param => {
                    self.selected_parameter.item_list = self.selected_function.item_list[self.selected_function.item_index].next;
                    self.select_param();
                }
                TuiState::Value => {}
            }
        }
        /*
        if new_state == TuiState::Param {
            self.query_current_value();
        }
        */
    }

    /** Queries the current value of the selected parameter, since we don't keep a local copy. */
    fn query_current_value(&self) {
        let function = &self.selected_function.item_list[self.selected_function.item_index];
        let function_id = if let ParameterValue::Int(x) = &self.selected_function.value { *x as usize } else { panic!() };
        let parameter = &self.selected_parameter.item_list[self.selected_parameter.item_index];
        let param_val = &self.selected_parameter.value;
        let param = SynthParam::new(function.item, function_id, parameter.item, *param_val);
        info!("query_current_value {:?}", param);
        self.sender.send(SynthMessage::ParamQuery(param)).unwrap();
    }

    /** Queries a samplebuffer from the synth engine to display. */
    fn get_waveform(&self) {
        let buffer = vec!(0.0; 100);
        self.sender.send(SynthMessage::WaveBuffer(buffer)).unwrap();
    }

    /** Select one of the items of functions or parameters. */
    fn select_item(&mut self, c: termion::event::Key) -> TuiState {
        let item = match self.state {
            TuiState::Function => &mut self.selected_function,
            TuiState::Param => &mut self.selected_parameter,
            _ => panic!()
        };
        info!("select_item {:?}", item.item_list[item.item_index].item);
        match c {
            Key::Up => {
                if item.item_index < item.item_list.len() - 1 {
                    item.item_index += 1;
                    self.select_param();
                }
                self.state
            }
            Key::Down => {
                if item.item_index > 0 {
                    item.item_index -= 1;
                    self.select_param();
                }
                self.state
            }
            Key::Left => previous(self.state),
            Key::Right => next(self.state),
            Key::Char('\n') => TuiState::Function,
            _ => {
                self.select_by_key(c)
            }
        }
    }

    /** Directly select an item by it's assigned keyboard shortcut. */
    fn select_by_key(&mut self, c: termion::event::Key) -> TuiState {
        let item = match self.state {
            TuiState::Function | TuiState::FunctionIndex=> &mut self.selected_function,
            TuiState::Param | TuiState::Value => &mut self.selected_parameter,
        };
        info!("select_by_key {:?}", c);
        for (count, f) in item.item_list.iter().enumerate() {
            if f.key == c {
                item.item_index = count;
                self.select_param();
                return next(self.state);
            }
        }
        self.state
    }

    /** Construct the value of the selected item.
     *
     * Supports entering the value by
     * - Direct ascii input of the number
     * - Adjusting current value with Up or Down keys
     */
    fn get_value(&mut self, c: termion::event::Key) -> TuiState {
        let item = match self.state {
            TuiState::FunctionIndex => &mut self.selected_function,
            TuiState::Value => &mut self.selected_parameter,
            _ => panic!()
        };
        let temp_string = &mut self.temp_string;
        info!("get_value {:?}", c);
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
                                    next(self.state) // Can't add another digit, accept value as final and move on
                                } else {
                                    self.state // Could add more digits, not finished yet
                                }
                            },
                            '\n' => next(self.state),
                            _ => {self.state = previous(self.state); return self.select_by_key(c)}
                        }
                    }
                    Key::Up        => { current += 1; self.state },
                    Key::Down      => { if current > 0 { current -= 1; } self.state },
                    Key::Left => previous(self.state),
                    Key::Right => next(self.state),
                    _ => next(self.state),
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
                                self.state
                            },
                            '\n' => next(self.state),
                            //_ => previous(self.state),
                            _ => {self.state = previous(self.state); return self.select_by_key(c)}
                        }
                    }
                    Key::Up        => { current += 1.0; self.state },
                    Key::Down      => { current -= 1.0; self.state },
                    Key::Left => previous(self.state),
                    Key::Backspace => {
                        let len = temp_string.len();
                        if len > 0 {
                            temp_string.pop();
                            if len >= 1 {
                                let value = temp_string.parse();
                                current = if let Ok(x) = value { x } else { current };
                                self.state
                            } else {
                                current = 0.0;
                                previous(self.state)
                            }
                        } else {
                            previous(self.state)
                        }
                    }
                    _ => previous(self.state)
                };
                Tui::update_value(item, &ParameterValue::Float(current), temp_string);
                new_state
            }
            ValueRange::ChoiceRange(choice_list) => {
                let mut current = if let ParameterValue::Choice(x) = val { x } else { panic!() };
                let new_state = match c {
                    Key::Up        => {current += 1; self.state },
                    Key::Down      => {if current > 0 { current -= 1 }; self.state },
                    Key::Left => {
                        previous(self.state)
                    }
                    Key::Char('\n') => next(self.state),
                    //_ => self.state
                    _ => {self.state = previous(self.state); return self.select_by_key(c)}
                };
                Tui::update_value(item, &ParameterValue::Choice(current), temp_string);
                new_state
            }
            _ => TuiState::Value
        }
    }

    /** Store the value read from the input in the parameter. */
    fn update_value(item: &mut SelectedItem, val: &ParameterValue, temp_string: &mut String) {
        info!("update_value {:?} to {:?}", item, val);
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

    /* Select the parameter chosen by input. */
    fn select_param(&mut self) {
        let item = match self.state {
            TuiState::Function | TuiState::FunctionIndex => &mut self.selected_function,
            TuiState::Param | TuiState::Value => &mut self.selected_parameter,
        };
        info!("select_param {:?}", item.item_list[item.item_index].item);
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
        if self.state == TuiState::Param {
            self.query_current_value();
        }
    }

    /* Send an updated value to the synth engine. */
    fn send_event(&self) {
        let function = &self.selected_function.item_list[self.selected_function.item_index];
        let function_id = if let ParameterValue::Int(x) = &self.selected_function.value { *x as usize } else { panic!() };
        let parameter = &self.selected_parameter.item_list[self.selected_parameter.item_index];
        let param_val = &self.selected_parameter.value;
        let param = SynthParam::new(function.item, function_id, parameter.item, *param_val);
        info!("send_event {:?}", param);
        self.sender.send(SynthMessage::Param(param)).unwrap();
    }

    /* ====================================================================== */

    /** Display the UI. */
    fn display(&self) {
        let mut x_pos: u16 = 1;
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
        //self.display_waveform();
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

    fn display_options(&self, x_pos: u16) {
        if self.state == TuiState::Function {
            let mut y_item = 2;
            let list = self.selected_function.item_list;
            for item in list.iter() {
                let k = if let Key::Char(c) = item.key { c } else { panic!(); };
                print!("{}{} - {}", cursor::Goto(x_pos, y_item), k, item.item);
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
                let k = if let Key::Char(c) = item.key { c } else { panic!(); };
                print!("{}{} - {}", cursor::Goto(x_pos, y_item), k, item.item);
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
    fn display_waveform(&self) {
        print!("{}{}", color::Bg(Black), color::Fg(White));
        self.canvas.render(1, 10);
        print!("{}{}", color::Bg(Rgb(255, 255, 255)), color::Fg(Black));
    }
}
