use super::parameter::{Parameter, ParameterValue, ParamId, FunctionId, SynthParam, ValueRange, Selection, SelectedItem, FUNCTIONS, OSC_PARAMS, MOD_SOURCES, MOD_TARGETS};
use super::{MidiMessage, MessageType};
use super::{UiMessage, SynthMessage};
use super::Canvas;
use super::Float;

use crossbeam_channel::{Sender, Receiver};
use log::{info, trace, warn};
use serde::{Serialize, Deserialize};
use termion::{clear, color, cursor};
use termion::color::{Black, White, LightWhite, Reset, Rgb};
use termion::event::Key;

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
    ValueSubFunction,
    ValueSubFunctionIndex,
    ValueSubParam,
}

enum ReturnCode {
    KeyConsumed,   // Key has been used, but value is not updated yet
    ValueUpdated,  // Key has been used and value has changed to a valid value
    ValueComplete, // Value has changed and will not be updated again
    KeyMissmatch,  // Key has not been used
    Cancel,        // Cancel current operation and go to previous state
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
        ValueSubFunction => ValueSubFunctionIndex,
        ValueSubFunctionIndex => ValueSubParam,
        ValueSubParam => ValueSubFunction,
    }
}

fn previous(current: TuiState) -> TuiState {
    use TuiState::*;
    match current {
        Function => Function,
        FunctionIndex => Function,
        Param => FunctionIndex,
        Value => Param,
        ValueSubFunction => Param,
        ValueSubFunctionIndex => ValueSubFunction,
        ValueSubParam => ValueSubFunctionIndex,
    }
}

pub struct Tui {
    // Function selection
    state: TuiState,
    sender: Sender<SynthMessage>,
    ui_receiver: Receiver<UiMessage>,

    // TUI handling
    selected_function: SelectedItem,
    selected_param: SelectedItem,
    selected_sub_function: SelectedItem,
    selected_sub_param: SelectedItem,

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
        let selected_function = SelectedItem{item_list: &FUNCTIONS, item_index: 0, value: ParameterValue::Int(1)};
        let selected_param = SelectedItem{item_list: &OSC_PARAMS, item_index: 0, value: ParameterValue::Int(1)};
        let selected_sub_function = SelectedItem{item_list: &MOD_SOURCES, item_index: 0, value: ParameterValue::Int(1)};
        let selected_sub_param = SelectedItem{item_list: &MOD_SOURCES, item_index: 0, value: ParameterValue::Int(1)};
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
            selected_function,
            selected_param,
            selected_sub_function,
            selected_sub_param,
            sync_counter,
            idle,
            busy,
            min_idle,
            max_busy,
            canvas,
            temp_string
        }
    }

    /** Start input handling thread.
     *
     * This thread receives messages from the terminal, the MIDI port, the
     * synth engine and the audio engine.
     */
    pub fn run(mut tui: Tui) -> std::thread::JoinHandle<()> {
        let handler = spawn(move || {
            loop {
                let msg = tui.ui_receiver.recv().unwrap();
                match msg {
                    UiMessage::Midi(m)  => tui.handle_midi_event(m),
                    UiMessage::Key(m) => tui.handle_user_input(m),
                    UiMessage::Param(m) => tui.handle_synth_param(m),
                    UiMessage::SampleBuffer(m, p) => tui.handle_samplebuffer(m, p),
                    UiMessage::EngineSync(idle, busy) => tui.handle_engine_sync(idle, busy),
                };
            }
        });
        handler
    }

    /* MIDI message received */
    fn handle_midi_event(&mut self, m: MidiMessage) {
        match m.get_message_type() {
            MessageType::ControlChg => {
                if m.param == 0x01 { // ModWheel
                    self.handle_control_change(m.value as i64);
                }
            },
            _ => ()
        }
    }

    /* Evaluate the MIDI control change message (ModWheel) */
    fn handle_control_change(&mut self, val: i64) {
        match self.state {
            TuiState::Function => self.change_state(TuiState::FunctionIndex),
            TuiState::FunctionIndex => (),
            TuiState::Param => self.change_state(TuiState::Value),
            TuiState::Value => (),
            TuiState::ValueSubFunction => self.change_state(TuiState::ValueSubFunctionIndex),
            TuiState::ValueSubFunctionIndex => (),
            TuiState::ValueSubParam => (),
        }
        let item = &mut self.selected_param;
        match item.item_list[item.item_index].val_range {
            ValueRange::IntRange(min, max) => {
                let inc: Float = (max - min) as Float / 127.0;
                let value = min + (val as Float * inc) as i64;
                Tui::update_value(item, ParameterValue::Int(value), &mut self.temp_string);
            }
            ValueRange::FloatRange(min, max) => {
                let inc: Float = (max - min) / 127.0;
                let value = min + val as Float * inc;
                Tui::update_value(item, ParameterValue::Float(value), &mut self.temp_string);
            }
            ValueRange::ChoiceRange(choice_list) => {
                let inc: Float = choice_list.len() as Float / 127.0;
                let value = (val as Float * inc) as i64;
                Tui::update_value(item, ParameterValue::Choice(value as usize), &mut self.temp_string);
            }
            _ => ()
        }
        self.send_event();
    }

    /* Received a queried parameter value from the synth engine. */
    fn handle_synth_param(&mut self, m: SynthParam) {
        info!("handle_synth_param {} = {:?}", self.selected_param.item_list[self.selected_param.item_index].item, m);
        let item = &mut self.selected_param;
        Tui::update_value(item, m.value, &mut self.temp_string);
    }

    /* Received a buffer with samples from the synth engine. */
    fn handle_samplebuffer(&mut self, m: Vec<Float>, p: SynthParam) {
        self.canvas.clear();
        match p.function {
            Parameter::Oscillator => {
                self.canvas.plot(&m, -1.0, 1.0);
            }
            Parameter::Envelope => {
                self.canvas.plot(&m, 0.0, 1.0);
            }
            _ => {}
        }
    }

    /* Received a sync signal from the audio engine.
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
            self.query_samplebuffer();
        }
    }

    /* Received a keyboard event from the terminal. */
    fn handle_user_input(&mut self, c: termion::event::Key) {
        let mut key_consumed = false;

        while !key_consumed {
            info!("handle_user_input {:?}", c);
            key_consumed = true;
            let new_state = match self.state {

                // Select the function group to edit (Oscillator, Envelope, ...)
                TuiState::Function => {
                    match Tui::select_item(&mut self.selected_function, c) {
                        ReturnCode::KeyConsumed | ReturnCode::ValueUpdated  => self.state,       // Selection updated
                        ReturnCode::KeyMissmatch | ReturnCode::Cancel       => self.state,       // Ignore key that doesn't match a selection
                        ReturnCode::ValueComplete                           => next(self.state), // Function selected
                    }
                },

                // Select which item in the function group to edit (Oscillator 1, 2, 3, ...)
                TuiState::FunctionIndex => {
                    match Tui::get_value(&mut self.selected_function, c, &mut self.temp_string) {
                        ReturnCode::KeyConsumed   => self.state,           // Key has been used, but value hasn't changed
                        ReturnCode::ValueUpdated  => self.state,           // Selection not complete yet
                        ReturnCode::ValueComplete => {                     // Parameter has been selected
                            self.selected_param.item_list = self.selected_function.item_list[self.selected_function.item_index].next;
                            Tui::select_param(&mut self.selected_param);
                            self.query_current_value();
                            next(self.state)
                        },
                        ReturnCode::KeyMissmatch  => self.state,           // Ignore unmatched keys
                        ReturnCode::Cancel        => previous(self.state), // Abort function index selection
                    }
                },

                // Select the parameter of the function to edit (Waveshape, Frequency, ...)
                TuiState::Param => {
                    match Tui::select_item(&mut self.selected_param, c) {
                        ReturnCode::KeyConsumed   => self.state,           // Value has changed, but not complete yet
                        ReturnCode::ValueUpdated  => {                     // Pararmeter selection updated
                            Tui::select_param(&mut self.selected_param);
                            self.query_current_value();
                            self.state
                        },
                        ReturnCode::ValueComplete => {                     // Prepare to read the value
                            Tui::select_param(&mut self.selected_param);
                            self.query_current_value();
                            next(self.state)
                        },
                        ReturnCode::KeyMissmatch  => self.state,           // Ignore invalid key
                        ReturnCode::Cancel        => previous(self.state), // Cancel parameter selection
                    }
                },

                // Select the parameter value
                TuiState::Value => {
                    // Hack: For modulator settings, we need to pass in a different struct, since
                    // that requires additional submenus.
                    let item = &mut self.selected_param;
                    match Tui::get_value(item, c, &mut self.temp_string) {
                        ReturnCode::KeyConsumed   => self.state,
                        ReturnCode::ValueUpdated  => { // Value has changed to a valid value, update synth
                            self.send_event();
                            self.state
                        },
                        ReturnCode::ValueComplete => previous(self.state), // Value has changed and will not be updated again
                        ReturnCode::KeyMissmatch  => {
                            // Key can't be used for value, so it probably is the short cut for a
                            // different parameter. Switch to parameter state and try again.
                            key_consumed = false;
                            previous(self.state)
                        },
                        ReturnCode::Cancel => {
                            // Stop updating the value, back to parameter selection
                            previous(self.state)
                        }
                    }
                }

                TuiState::ValueSubFunction => {
                    match Tui::select_item(&mut self.selected_sub_function, c) {
                        ReturnCode::KeyConsumed | ReturnCode::ValueUpdated  => self.state,       // Selection updated
                        ReturnCode::KeyMissmatch                            => self.state,       // Ignore key that doesn't match a selection
                        ReturnCode::ValueComplete                           => next(self.state), // Function selected
                        ReturnCode::Cancel => {
                            // Stop updating the value, back to parameter selection
                            previous(self.state)
                        }
                    }
                },

                // Select which item in the function group to edit (Oscillator 1, 2, 3, ...)
                TuiState::ValueSubFunctionIndex => {
                    match Tui::get_value(&mut self.selected_sub_function, c, &mut self.temp_string) {
                        ReturnCode::KeyConsumed   => self.state,           // Key has been used, but value hasn't changed
                        ReturnCode::ValueUpdated  => self.state,           // Selection not complete yet
                        ReturnCode::ValueComplete => {                     // Parameter has been selected
                            let item = &mut self.selected_param;
                            match item.item_list[item.item_index].item {
                                Parameter::Source => {
                                    // Finished getting the modulation source
                                    let item = &self.selected_sub_function;
                                    let mut func_id = if let ParameterValue::Int(x) = item.value { x as usize } else { panic!() };
                                    if func_id == 0 { func_id = 1; }
                                    self.selected_param.value = ParameterValue::Function(FunctionId{function: item.item_list[item.item_index].item, function_id: func_id});
                                    self.send_event();
                                    previous(self.state)
                                }
                                Parameter::Target => {
                                    let mut item = &mut self.selected_sub_param;
                                    item.item_list = item.item_list[item.item_index].next;
                                    Tui::select_param(item);
                                    self.query_current_value();
                                    next(self.state)
                                }
                                Parameter::Amount => previous(self.state),
                                _ => panic!("{:?}", item.item_list[item.item_index].item),
                            }

                        },
                        ReturnCode::KeyMissmatch  => self.state,           // Ignore unmatched keys
                        ReturnCode::Cancel        => previous(self.state), // Abort function index selection
                    }
                },

                // Select the parameter of the function to edit (Waveshape, Frequency, ...)
                TuiState::ValueSubParam => {
                    match Tui::select_item(&mut self.selected_sub_param, c) {
                        ReturnCode::KeyConsumed   => self.state,           // Value has changed, but not complete yet
                        ReturnCode::ValueUpdated  => {                     // Pararmeter selection updated
                            Tui::select_param(&mut self.selected_sub_param);
                            self.query_current_value();
                            self.state
                        },
                        ReturnCode::ValueComplete => {
                            let func = &self.selected_sub_function;
                            let func_id = if let ParameterValue::Int(x) = func.value { x as usize } else { panic!() };
                            let param = &self.selected_sub_param;
                            self.selected_param.value = ParameterValue::Param(ParamId{function: func.item_list[func.item_index].item,
                                                                                         function_id: func_id,
                                                                                         parameter: param.item_list[param.item_index].item});
                            self.send_event();
                            next(self.state)
                        },
                        ReturnCode::KeyMissmatch  => self.state,           // Ignore invalid key
                        ReturnCode::Cancel        => previous(self.state), // Cancel parameter selection
                    }
                },

            };
            self.change_state(new_state);
        }
    }

    /* Change the state of the input state machine. */
    fn change_state(&mut self, mut new_state: TuiState) {
        if new_state != self.state {
            match new_state {
                TuiState::Function => {
                    // We are probably selecting a different function than
                    // before, so we should start the parameter list at the
                    // beginning to avoid out-of-bound errors.
                    self.selected_param.item_index = 0;
                }
                TuiState::FunctionIndex => {}
                TuiState::Param => {}
                TuiState::Value => {
                    // For modulation parameters, we need to enter a special
                    // sub state
                    let f = &self.selected_function;
                    if f.item_list[f.item_index].item == Parameter::Modulation {
                        let p = &self.selected_param;
                        match p.item_list[p.item_index].item {
                            Parameter::Source => {
                                new_state = TuiState::ValueSubFunction;
                                self.selected_sub_function.item_list = &MOD_SOURCES;
                            },
                            Parameter::Target =>  {
                                new_state = TuiState::ValueSubFunction;
                                self.selected_sub_function.item_list = &MOD_TARGETS;
                            },
                            Parameter::Amount => (),
                            _ => panic!(),
                        }
                        let val = if let ParameterValue::Param(p) = f.value { p } else { panic!("{:?}", f.value) };
                        self.selected_sub_function.item_index = val.function_id;
                    }
                }
                TuiState::ValueSubFunction => {}
                TuiState::ValueSubFunctionIndex => {
                    let item = &self.selected_sub_function;
                    if item.item_list[item.item_index].item == Parameter::Modulation {
                        let item = &self.selected_sub_param;
                        match item.item_list[item.item_index].item {
                            Parameter::Source => new_state = TuiState::ValueSubFunction,
                            Parameter::Target =>  new_state = TuiState::ValueSubParam,
                            Parameter::Amount => (),
                            _ => panic!(),
                        }
                    }
                }
                TuiState::ValueSubParam => {}
            }
            info!("change_state {} -> {}", self.state, new_state);
            self.state = new_state;
        }
    }

    /* Queries the current value of the selected parameter from the synth engine, since we don't
     * keep a local copy. */
    fn query_current_value(&self) {
        let function = &self.selected_function.item_list[self.selected_function.item_index];
        let function_id = if let ParameterValue::Int(x) = &self.selected_function.value { *x as usize } else { panic!() };
        let parameter = &self.selected_param.item_list[self.selected_param.item_index];
        let param_val = &self.selected_param.value;
        let param = SynthParam::new(function.item, function_id, parameter.item, *param_val);
        info!("query_current_value {:?}", param);
        self.sender.send(SynthMessage::ParamQuery(param)).unwrap();
    }

    /* Queries a samplebuffer from the synth engine to display.
     *
     * The samplebuffer can contain wave shapes or envelopes.
     */
    fn query_samplebuffer(&self) {
        let buffer = vec!(0.0; 100);
        let function = &self.selected_function.item_list[self.selected_function.item_index];
        let function_id = if let ParameterValue::Int(x) = &self.selected_function.value { *x as usize } else { panic!() };
        let parameter = &self.selected_param.item_list[self.selected_param.item_index];
        let param_val = &self.selected_param.value;
        let param = SynthParam::new(function.item, function_id, parameter.item, *param_val);
        self.sender.send(SynthMessage::SampleBuffer(buffer, param)).unwrap();
    }

    /* Select one of the items of functions or parameters.
     *
     * Called when a new user input is received and we're in the right state for function selection.
     */
    fn select_item(item: &mut SelectedItem, c: termion::event::Key) -> ReturnCode {
        let result = match c {
            Key::Up => {
                if item.item_index < item.item_list.len() - 1 {
                    item.item_index += 1;
                }
                ReturnCode::ValueUpdated
            },
            Key::Down => {
                if item.item_index > 0 {
                    item.item_index -= 1;
                }
                ReturnCode::ValueUpdated
            },
            Key::Left | Key::Backspace => ReturnCode::Cancel,
            Key::Right => ReturnCode::ValueComplete,
            Key::Char('\n') => ReturnCode::ValueComplete,
            Key::Char(c) => {
                for (count, f) in item.item_list.iter().enumerate() {
                    if f.key == c {
                        item.item_index = count;
                        return ReturnCode::ValueComplete;
                    }
                }
                ReturnCode::KeyConsumed
            },
            _ => ReturnCode::KeyConsumed
        };
        info!("select_item {:?}", item.item_list[item.item_index].item);
        result
    }

    /* Construct the value of the selected item.
     *
     * Supports entering the value by
     * - Direct ascii input of the number
     * - Adjusting current value with Up or Down keys
     */
    fn get_value(item: &mut SelectedItem, c: termion::event::Key, temp_string: &mut String) -> ReturnCode {
        info!("get_value {:?}", item.item_list[item.item_index].item);
        match item.item_list[item.item_index].val_range {
            ValueRange::IntRange(min, max) => {
                let mut current = if let ParameterValue::Int(x) = item.value { x } else { panic!() };
                let result = match c {
                    Key::Char(x) => {
                        match x {
                            // TODO: This doesn't work well, switch to using the temp_string here as well.
                            '0' | '1' | '2' | '3' | '4' | '5' | '6' | '7' | '8' | '9' => {
                                let y = x as i64 - '0' as i64;
                                let val_digit_added = current * 10 + y;
                                if val_digit_added > max {
                                    current = y; // Can't add another digit, replace current value with new one
                                } else {
                                    current = val_digit_added;
                                }
                                item.value = ParameterValue::Int(current);
                                if current * 10 > max {
                                    ReturnCode::ValueComplete // Can't add another digit, accept value as final and move on
                                } else {
                                    ReturnCode::KeyConsumed   // Could add more digits, not finished yet
                                }
                            },
                            '\n' => ReturnCode::ValueComplete,
                            _ => ReturnCode::KeyMissmatch,
                        }
                    }
                    Key::Up        => { current += 1; ReturnCode::ValueUpdated },
                    Key::Down      => if current > min { current -= 1; ReturnCode::ValueUpdated } else { ReturnCode::KeyConsumed },
                    Key::Left      => ReturnCode::Cancel,
                    Key::Right     => ReturnCode::ValueComplete,
                    Key::Backspace => ReturnCode::Cancel,
                    _              => ReturnCode::ValueComplete,
                };
                match result {
                    ReturnCode::ValueUpdated | ReturnCode::ValueComplete => Tui::update_value(item, ParameterValue::Int(current), temp_string),
                    _ => (),
                }
                result
            },
            ValueRange::FloatRange(min, max) => {
                let mut current = if let ParameterValue::Float(x) = item.value { x } else { panic!() };
                let result = match c {
                    Key::Char(x) => {
                        match x {
                            '0' | '1' | '2' | '3' | '4' | '5' | '6' | '7' | '8' | '9' | '.' => {
                                temp_string.push(x);
                                let value: Result<Float, ParseFloatError> = temp_string.parse();
                                current = if let Ok(x) = value { x } else { current };
                                ReturnCode::KeyConsumed
                            },
                            '\n' => ReturnCode::ValueComplete,
                            _ => ReturnCode::KeyMissmatch,
                        }
                    }
                    Key::Up        => { current += 1.0; ReturnCode::ValueUpdated },
                    Key::Down      => { current -= 1.0; ReturnCode::ValueUpdated },
                    Key::Left      => ReturnCode::Cancel,
                    Key::Right     => ReturnCode::ValueComplete,
                    Key::Backspace => {
                        let len = temp_string.len();
                        if len > 0 {
                            temp_string.pop();
                            if len >= 1 {
                                let value = temp_string.parse();
                                current = if let Ok(x) = value { x } else { current };
                            } else {
                                current = 0.0;
                            }
                        }
                        ReturnCode::KeyConsumed
                    },
                    _ => ReturnCode::KeyMissmatch,
                };
                match result {
                    ReturnCode::ValueUpdated | ReturnCode::ValueComplete => Tui::update_value(item, ParameterValue::Float(current), temp_string),
                    _ => (),
                }
                result
            },
            ValueRange::ChoiceRange(choice_list) => {
                let mut current = if let ParameterValue::Choice(x) = item.value { x } else { panic!() };
                let result = match c {
                    Key::Up         => {current += 1; ReturnCode::ValueUpdated },
                    Key::Down       => if current > 0 { current -= 1; ReturnCode::ValueUpdated } else { ReturnCode::KeyConsumed },
                    Key::Left | Key::Backspace => ReturnCode::Cancel,
                    Key::Right      => ReturnCode::ValueComplete,
                    Key::Char('\n') => ReturnCode::ValueComplete,
                    _ => ReturnCode::KeyMissmatch,
                };
                match result {
                    ReturnCode::ValueUpdated | ReturnCode::ValueComplete => Tui::update_value(item, ParameterValue::Choice(current), temp_string),
                    _ => (),
                }
                result
            },
            ValueRange::ModRange(choice_list) => {
                Tui::select_item(item, c)
            }
            _ => panic!(),
        }
    }

    /* Select the parameter chosen by input. */
    fn select_param(item: &mut SelectedItem) {
        info!("select_param {:?}", item.item_list[item.item_index].item);
        let param = item.item_list[item.item_index].item;

        // The value in the selected parameter needs to point to the right type.
        // Initialize it with the minimum.
        let val_range = &item.item_list[item.item_index].val_range;
        item.value = match val_range {
            ValueRange::IntRange(min, _) => {
                ParameterValue::Int(*min)
            }
            ValueRange::FloatRange(min, _) => {
                ParameterValue::Float(*min)
            }
            ValueRange::ChoiceRange(choice_list) => {
                ParameterValue::Choice(0)
            }
            ValueRange::ModRange(choice_list) => {
                match param {
                    Parameter::Source => ParameterValue::Function(FunctionId{..Default::default()}),
                    Parameter::Target => ParameterValue::Param(ParamId{..Default::default()}),
                    _ => panic!(),
                }
            }
            _ => ParameterValue::NoValue
        }
    }

    /* Store a new value in the selected parameter. */
    fn update_value(item: &mut SelectedItem, val: ParameterValue, temp_string: &mut String) {
        info!("update_value item: {:?}, value: {:?}", item.item_list[item.item_index].item, val);
        match item.item_list[item.item_index].val_range {
            ValueRange::IntRange(min, max) => {
                let mut val = if let ParameterValue::Int(x) = val { x } else { panic!(); };
                if val > max {
                    val = max;
                }
                if val < min {
                    val = min;
                }
                item.value = ParameterValue::Int(val.try_into().unwrap());
            }
            ValueRange::FloatRange(min, max) => {
                let mut val = if let ParameterValue::Float(x) = val { x } else { panic!(); };
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
                let mut val = if let ParameterValue::Choice(x) = val { x as usize } else { panic!("{:?}", val); };
                if val >= selection_list.len() {
                    val = selection_list.len() - 1;
                }
                item.value = ParameterValue::Choice(val);
            }
            ValueRange::ModRange(selection_list) => {
                match val {
                    ParameterValue::Function(x) => {
                        item.value = val;
                    }
                    ParameterValue::Param(x) => {
                        item.value = val;
                    }
                    _ => panic!(),
                }
                item.value = val;
            }
            ValueRange::NoRange => {}
        };
    }

    /* Send an updated value to the synth engine. */
    fn send_event(&self) {
        let function = &self.selected_function.item_list[self.selected_function.item_index];
        let function_id = if let ParameterValue::Int(x) = &self.selected_function.value { *x as usize } else { panic!() };
        let parameter = &self.selected_param.item_list[self.selected_param.item_index];
        let param_val = &self.selected_param.value;
        let param = SynthParam::new(function.item, function_id, parameter.item, *param_val);
        //info!("send_event {:?}", param);
        self.sender.send(SynthMessage::Param(param)).unwrap();
    }

    /* ====================================================================== */

    /** Display the UI. */
    fn display(&self) {
        let mut x_pos: u16 = 1;
        let mut display_state = TuiState::Function;
        print!("{}{}", clear::All, cursor::Goto(1, 1));
        loop {
            match display_state {
                TuiState::Function => {
                    Tui::display_function(&self.selected_function, self.state == TuiState::Function);
                }
                TuiState::FunctionIndex => {
                    Tui::display_function_index(&self.selected_function, self.state == TuiState::FunctionIndex);
                    x_pos = 12;
                }
                TuiState::Param => {
                    Tui::display_param(&self.selected_param, self.state == TuiState::Param);
                    x_pos = 14;
                }
                _ => break,
            }
            if display_state == self.state {
                break;
            }
            display_state = next(display_state);
        }
        match self.state {
            TuiState::Value => {
                    Tui::display_value(&self.selected_param, self.state == TuiState::Value);
                    x_pos = 23;
            }
            TuiState::ValueSubFunction | TuiState::ValueSubFunctionIndex | TuiState::ValueSubParam => {
                display_state = TuiState::ValueSubFunction;
                loop {
                    match display_state {
                        TuiState::ValueSubFunction => {
                            Tui::display_function(&self.selected_sub_function, true);
                            x_pos = 30;
                        }
                        TuiState::ValueSubFunctionIndex => {
                            Tui::display_function_index(&self.selected_sub_function, true);
                            x_pos = 32;
                        }
                        TuiState::ValueSubParam => {
                            Tui::display_param(&self.selected_sub_param, true);
                        }
                        _ => (),
                    }
                    if display_state == self.state {
                        break;
                    }
                    display_state = next(display_state);
                }
            }
            _ => ()
        }
        //print!("{}", clear::UntilNewline);
        self.display_options(x_pos);
        self.display_samplebuff();
        io::stdout().flush().ok();
    }

    fn display_function(func: &SelectedItem, selected: bool) {
        if selected {
            print!("{}{}", color::Bg(LightWhite), color::Fg(Black));
        } else {
            print!("{}{}", color::Bg(Rgb(255, 255, 255)), color::Fg(Black));
        }
        print!("{}", func.item_list[func.item_index].item);
        if selected {
            print!("{}{}", color::Bg(Rgb(255, 255, 255)), color::Fg(Black));
        }
    }

    fn display_function_index(func: &SelectedItem, selected: bool) {
        if selected {
            print!("{}{}", color::Bg(LightWhite), color::Fg(Black));
        }
        let function_id = if let ParameterValue::Int(x) = &func.value { *x as usize } else { panic!() };
        print!(" {}", function_id);
        if selected {
            print!("{}{}", color::Bg(Rgb(255, 255, 255)), color::Fg(Black));
        }
    }

    fn display_param(param: &SelectedItem, selected: bool) {
        if selected {
            print!("{}{}", color::Bg(LightWhite), color::Fg(Black));
        }
        print!(" {}", param.item_list[param.item_index].item);
        if selected {
            print!("{}{}", color::Bg(Rgb(255, 255, 255)), color::Fg(Black));
        }
    }

    fn display_value(param: &SelectedItem, selected: bool) {
        if selected {
            print!("{}{}", color::Bg(LightWhite), color::Fg(Black));
        }
        match param.value {
            ParameterValue::Int(x) => print!(" {}", x),
            ParameterValue::Float(x) => print!(" {}", x),
            ParameterValue::Choice(x) => {
                let item = &param.item_list[param.item_index];
                let range = &item.val_range;
                let selection = if let ValueRange::ChoiceRange(list) = range { list } else { panic!() };
                let item = selection[x].item;
                print!(" {}", item);
            },
            ParameterValue::Function(x) => {
                print!(" {:?}", x.function);
            },
            ParameterValue::Param(x) => {
                print!(" {:?}", x.function);
            },
            _ => ()
        }
        if selected {
            print!("{}{}", color::Bg(Rgb(255, 255, 255)), color::Fg(Black));
        }
    }

    fn display_options(&self, x_pos: u16) {
        if self.state == TuiState::Function {
            let mut y_item = 2;
            let list = self.selected_function.item_list;
            for item in list.iter() {
                print!("{}{} - {}", cursor::Goto(x_pos, y_item), item.key, item.item);
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
            let list = self.selected_param.item_list;
            for item in list.iter() {
                print!("{}{} - {}", cursor::Goto(x_pos, y_item), item.key, item.item);
                y_item += 1;
            }
        }
        if self.state == TuiState::Value {
            let range = &self.selected_param.item_list[self.selected_param.item_index].val_range;
            match range {
                ValueRange::IntRange(min, max) => print!("{}{} - {}", cursor::Goto(x_pos, 2), min, max),
                ValueRange::FloatRange(min, max) => print!("{}{} - {}", cursor::Goto(x_pos, 2), min, max),
                ValueRange::ChoiceRange(list) => print!("{}1 - {}", cursor::Goto(x_pos, 2), list.len()),
                ValueRange::ModRange(list) => {
                    let mut y_item = 2;
                    let list = self.selected_sub_param.item_list;
                    for item in list.iter() {
                        print!("{}{} - {}", cursor::Goto(x_pos, y_item), item.key, item.item);
                        y_item += 1;
                    }
                }
                ValueRange::NoRange => ()
            }
        }
    }

    fn display_samplebuff(&self) {
        print!("{}{}", color::Bg(Black), color::Fg(White));
        self.canvas.render(1, 10);
        print!("{}{}", color::Bg(Rgb(255, 255, 255)), color::Fg(Black));
    }
}
