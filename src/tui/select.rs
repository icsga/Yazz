use super::Float;
use super::{FunctionId, ParamId};
use super::{Parameter, ParameterValue, SynthParam, ValueRange, MenuItem, FUNCTIONS, OSC_PARAMS, MOD_SOURCES, MOD_TARGETS};
use super::MidiLearn;
use super::{SoundData, SoundPatch};
use super::{StateMachine, SmEvent, SmResult};

use log::{info, trace, warn};
use termion::event::Key;

use std::cell::RefCell;
use std::convert::TryInto;
use std::fmt::{self, Debug};
use std::num::ParseFloatError;
use std::num::ParseIntError;
use std::rc::Rc;

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum SelectorState {
    Function,
    FunctionIndex,
    Param,
    Value,
    MidiLearn,
    ValueFunction,
    ValueFunctionIndex,
    ValueParam,
}

impl fmt::Display for SelectorState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

pub fn next(current: SelectorState) -> SelectorState {
    use SelectorState::*;
    match current {
        Function => FunctionIndex,
        FunctionIndex => Param,
        Param => Value,
        Value => MidiLearn,
        MidiLearn => ValueFunction,
        ValueFunction => ValueFunctionIndex,
        ValueFunctionIndex => ValueParam,
        ValueParam => ValueParam,
    }
}

#[derive(Debug)]
pub struct ItemSelection {
    pub item_list: &'static [MenuItem], // List of MenuItems to select from
    pub item_index: usize,              // Index into the MenuItem list to the chosen item
    pub value: ParameterValue,          // ID or value of the selected item
    pub temp_string: String,
}

impl ItemSelection {
    pub fn reset(&mut self) {
        self.item_index = 0;
        self.value = match self.item_list[0].val_range {
            ValueRange::Int(min, _) => ParameterValue::Int(min),
            ValueRange::Float(min, _, _) => ParameterValue::Float(min),
            ValueRange::Choice(_) => ParameterValue::Choice(0),
            ValueRange::Dynamic(_) => ParameterValue::Choice(0),
            ValueRange::Func(func_list) => ParameterValue::Function(FunctionId{function: func_list[0].item, function_id: 0}),
            ValueRange::Param(func_list) => ParameterValue::Function(FunctionId{function: func_list[0].item, function_id: 0}),
            ValueRange::NoRange => panic!(),
        };
    }
}

/* Return code from functions handling user input. */
#[derive(Debug, PartialEq)]
pub enum RetCode {
    KeyConsumed,   // Key has been used, but value is not updated yet
    ValueUpdated,  // Key has been used and value has changed to a valid value
    ValueComplete, // Value has changed and will not be updated again
    KeyMissmatch,  // Key has not been used
    Cancel,        // Cancel current operation and go to previous state
    Reset,         // Reset parameter selection to initial state (function selection)
}

pub enum SelectorEvent {
    Key(termion::event::Key), // A key has been pressed
    ControlChange(u64, u64),  // The input controller has been updated
    ValueChange(SynthParam),  // A sound parameter has changed outside of the selector (by MIDI controller)
}

#[derive(Debug)]
pub struct ParamSelector {
    value_changed: bool,
    pub state: SelectorState,
    pub func_selection: ItemSelection,
    pub param_selection: ItemSelection,
    pub value_func_selection: ItemSelection,
    pub value_param_selection: ItemSelection,
    pub value: ParameterValue,
    pub ml: MidiLearn,
    sound: Option<Rc<RefCell<SoundPatch>>>,
}

impl ParamSelector {
    pub fn new(function_list: &'static [MenuItem], mod_list: &'static [MenuItem]) -> ParamSelector {
        let mut func_selection = ItemSelection{
            item_list: function_list,
            item_index: 0,
            value: ParameterValue::Int(1),
            temp_string: String::new()};
        func_selection.reset();
        let mut param_selection = ItemSelection{item_list: function_list[0].next,
            item_index: 0,
            value: ParameterValue::Int(1),
            temp_string: String::new()};
        param_selection.reset();
        let mut value_func_selection = ItemSelection{
            item_list: mod_list,
            item_index: 0,
            value: ParameterValue::Int(1),
            temp_string: String::new()};
        value_func_selection.reset();
        let mut value_param_selection = ItemSelection{item_list: mod_list[0].next,
            item_index: 0,
            value: ParameterValue::Int(1),
            temp_string: String::new()};
        value_param_selection.reset();
        ParamSelector{value_changed: false,
                      state: SelectorState::Function,
                      func_selection: func_selection,
                      param_selection: param_selection,
                      value_func_selection: value_func_selection,
                      value_param_selection: value_param_selection,
                      value: ParameterValue::Int(0),
                      ml: MidiLearn::new(),
                      sound: Option::None,
        }
    }

    pub fn reset(&mut self) -> SmResult<ParamSelector, SelectorEvent> {
        self.func_selection.reset();
        self.param_selection.reset();
        SmResult::ChangeState(ParamSelector::state_function)
    }

    /* Received a keyboard event from the terminal.
     *
     * Return 
     * - true if value has changed and can be sent to the synth engine
     * - false if value has not changed
     */
    pub fn handle_user_input(&mut self,
                             sm: &mut StateMachine<ParamSelector, SelectorEvent>,
                             c: termion::event::Key,
                             sound: Rc<RefCell<SoundPatch>>) -> bool {
        info!("handle_user_input {:?} in state {:?}", c, self.state);
        self.sound = Option::Some(Rc::clone(&sound));
        self.value_changed = false;
        sm.handle_event(self, &SmEvent::Event(SelectorEvent::Key(c)));
        self.value_changed
    }

    /* Received a controller event.
     *
     * This event is used as a direct value control for the UI menu line.
     *
     * \todo This is the same as handle_user_input, consolidate
     */
    pub fn handle_control_input(&mut self,
                                sm: &mut StateMachine<ParamSelector, SelectorEvent>,
                                controller: u64,
                                value: u64,
                                sound: Rc<RefCell<SoundPatch>>) -> bool {
        info!("handle_control_input {:?} in state {:?}", value, self.state);
        self.sound = Option::Some(Rc::clone(&sound));
        self.value_changed = false;
        sm.handle_event(self, &SmEvent::Event(SelectorEvent::ControlChange(controller, value)));
        self.value_changed
    }

    // Select the function group to edit (Oscillator, Envelope, ...)
    pub fn state_function(self: &mut ParamSelector,
                      e: &SmEvent<SelectorEvent>)
    -> SmResult<ParamSelector, SelectorEvent> {
        match e {
            SmEvent::EnterState => {
                info!("state_function Enter");
                self.state = SelectorState::Function;
                SmResult::EventHandled
            }
            SmEvent::ExitState => {
                SmResult::EventHandled
            }
            SmEvent::Event(selector_event) => {
                match selector_event {
                    SelectorEvent::Key(c) => {
                        match ParamSelector::select_item(&mut self.func_selection, *c) {
                            RetCode::KeyConsumed   => SmResult::EventHandled, // Selection updated
                            RetCode::KeyMissmatch  => SmResult::EventHandled, // Ignore key that doesn't match a selection
                            RetCode::ValueUpdated  => SmResult::EventHandled, // Selection updated
                            RetCode::Cancel        => SmResult::EventHandled,
                            RetCode::ValueComplete => {
                                // Function selected
                                self.param_selection.item_list = self.func_selection.item_list[self.func_selection.item_index].next;
                                SmResult::ChangeState(ParamSelector::state_function_index)
                            },
                            RetCode::Reset         => self.reset(),
                        }
                    }
                    SelectorEvent::ControlChange(c, i) => SmResult::EventHandled,
                    _ => SmResult::EventHandled,
                }
            }
        }
    }

    // Select which item in the function group to edit (Oscillator 1, 2, 3, ...)
    fn state_function_index(self: &mut ParamSelector,
                            e: &SmEvent<SelectorEvent>)
    -> SmResult<ParamSelector, SelectorEvent> {
        match e {
            SmEvent::EnterState => {
                info!("state_function_index Enter");
                self.state = SelectorState::FunctionIndex;
                SmResult::EventHandled
            }
            SmEvent::ExitState => {
                self.func_selection.temp_string.clear();
                SmResult::EventHandled
            }
            SmEvent::Event(selector_event) => {
                match selector_event {
                    SelectorEvent::Key(c) => {
                        match ParamSelector::get_value(&mut self.func_selection, *c) {
                            RetCode::KeyConsumed   => SmResult::EventHandled, // Key has been used, but value hasn't changed
                            RetCode::KeyMissmatch  => SmResult::EventHandled, // Ignore unmatched keys
                            RetCode::ValueUpdated  => SmResult::EventHandled, // Selection not complete yet
                            RetCode::ValueComplete => {
                                // Parameter has been selected, fetch current value from sound
                                self.query_current_value();
                                SmResult::ChangeState(ParamSelector::state_parameter)
                            },
                            RetCode::Cancel        => SmResult::ChangeState(ParamSelector::state_function), // Abort function index selection
                            RetCode::Reset         => self.reset(),
                        }
                    }
                    SelectorEvent::ControlChange(ctrl, value) => SmResult::EventHandled,
                    _ => SmResult::EventHandled,
                }
            }
        }
    }

    // Select the parameter of the function to edit (Waveshape, Frequency, ...)
    fn state_parameter(self: &mut ParamSelector,
                       e: &SmEvent<SelectorEvent>)
    -> SmResult<ParamSelector, SelectorEvent> {
        match e {
            SmEvent::EnterState => {
                info!("state_parameter Enter");
                self.state = SelectorState::Param;
                SmResult::EventHandled
            }
            SmEvent::ExitState => {
                SmResult::EventHandled
            }
            SmEvent::Event(selector_event) => {
                match selector_event {
                    SelectorEvent::Key(c) => {
                        if *c == Key::PageUp {
                            self.increase_function_id();
                            return SmResult::EventHandled;
                        } else if *c == Key::PageDown {
                                self.decrease_function_id();
                                return SmResult::EventHandled;
                        } else {
                            match ParamSelector::select_item(&mut self.param_selection, *c) {
                                RetCode::KeyConsumed   => SmResult::EventHandled, // Value has changed, but not complete yet
                                RetCode::KeyMissmatch  => SmResult::EventHandled, // Ignore invalid key
                                RetCode::ValueUpdated  => {                       // Pararmeter selection updated
                                    self.query_current_value();
                                    SmResult::EventHandled
                                },
                                RetCode::ValueComplete => {                       // Prepare to read the value
                                    self.query_current_value();
                                    match self.param_selection.value {
                                        ParameterValue::Function(id) => SmResult::ChangeState(ParamSelector::state_value_function),
                                        ParameterValue::Param(id) => SmResult::ChangeState(ParamSelector::state_value_function),
                                        ParameterValue::NoValue => panic!(),
                                        _ => SmResult::ChangeState(ParamSelector::state_value),
                                    }
                                    
                                },
                                RetCode::Cancel        => SmResult::ChangeState(ParamSelector::state_function_index), // Cancel parameter selection
                                RetCode::Reset         => self.reset(),
                            }
                        }
                    }
                    SelectorEvent::ControlChange(c, i) => SmResult::ChangeState(ParamSelector::state_value),
                    _ => SmResult::EventHandled,
                }
            }
        }
    }

    // Select the parameter value
    fn state_value(self: &mut ParamSelector,
                   e: &SmEvent<SelectorEvent>)
    -> SmResult<ParamSelector, SelectorEvent> {
        match e {
            SmEvent::EnterState => {
                info!("state_value Enter");
                self.state = SelectorState::Value;
                self.param_selection.temp_string.clear();
                SmResult::EventHandled
            }
            SmEvent::ExitState => {
                SmResult::EventHandled
            }
            SmEvent::Event(selector_event) => {
                match selector_event {
                    SelectorEvent::Key(c) => {
                        self.handle_key_in_state_value(c)
                    }
                    SelectorEvent::ControlChange(ctrl, value) => {
                        ParamSelector::set_control_value(&mut self.param_selection, *value);
                        self.value_changed = true;
                        SmResult::EventHandled
                    }
                    SelectorEvent::ValueChange(p) => {
                        let selected_param = self.get_param_id();
                        if p.equals(&selected_param) {
                            ParamSelector::update_value(&mut self.param_selection, p.value);
                        }
                        SmResult::EventHandled
                    }
                }
            }
        }
    }

    fn handle_key_in_state_value(&mut self, c: &termion::event::Key) -> SmResult<ParamSelector, SelectorEvent> {
        // Check for shortcut keys
        match *c {
            Key::Char('l') => return SmResult::ChangeState(ParamSelector::state_midi_learn),
            Key::PageUp => {
                self.increase_function_id();
                return SmResult::EventHandled;
            }
            Key::PageDown => {
                self.decrease_function_id();
                return SmResult::EventHandled;
            }
            _ => () // Pass on to code below
        }
        match ParamSelector::get_value(&mut self.param_selection, *c) {
            RetCode::KeyConsumed   => SmResult::EventHandled,
            RetCode::KeyMissmatch  => {
                // Key can't be used for value, so it probably is the short cut for a
                // different parameter. Switch to parameter state and try again.
                //
                // TODO: Push key back on stack
                //
                SmResult::ChangeState(ParamSelector::state_parameter)
            },
            RetCode::ValueUpdated  => {
                // Value has changed to a valid value, update synth
                self.value_changed = true;
                SmResult::EventHandled
            },
            RetCode::ValueComplete => {
                // Value has changed and will not be updated again
                self.value_changed = true;
                SmResult::ChangeState(ParamSelector::state_parameter)
            }
            RetCode::Cancel => SmResult::ChangeState(ParamSelector::state_parameter), // Stop updating the value, back to parameter selection
            RetCode::Reset => self.reset(),
        }
    }

    fn increase_function_id(&mut self) {
        ParamSelector::get_value(&mut self.func_selection, Key::Up);
        self.query_current_value();
    }

    fn decrease_function_id(&mut self) {
        ParamSelector::get_value(&mut self.func_selection, Key::Down);
        self.query_current_value();
    }

    fn state_midi_learn(self: &mut ParamSelector,
                        e: &SmEvent<SelectorEvent>)
    -> SmResult<ParamSelector, SelectorEvent> {
        match e {
            SmEvent::EnterState => {
                info!("state_midi_learn Enter");
                self.state = SelectorState::MidiLearn;
                self.ml.reset();
                SmResult::EventHandled
            }
            SmEvent::ExitState => {
                SmResult::EventHandled
            }
            SmEvent::Event(selector_event) => {
                match selector_event {
                    SelectorEvent::Key(c) => {
                        match c {
                            Key::Esc => SmResult::ChangeState(ParamSelector::state_value),
                            _ => SmResult::EventHandled
                        }
                    }
                    SelectorEvent::ControlChange(ctrl, value) => {
                        if self.ml.handle_controller(*ctrl, *value) {
                            SmResult::ChangeState(ParamSelector::state_value)
                        } else {
                            SmResult::EventHandled
                        }
                    }
                    _ => SmResult::EventHandled,
                }
            }
        }
    }

    // Select the function group to edit (Oscillator, Envelope, ...)
    fn state_value_function(self: &mut ParamSelector,
                            e: &SmEvent<SelectorEvent>)
    -> SmResult<ParamSelector, SelectorEvent> {
        match e {
            SmEvent::EnterState => {
                info!("state_value_function Enter");
                self.state = SelectorState::ValueFunction;
                SmResult::EventHandled
            }
            SmEvent::ExitState => {
                SmResult::EventHandled
            }
            SmEvent::Event(selector_event) => {
                match selector_event {
                    SelectorEvent::Key(c) => {
                        if *c == Key::PageUp {
                            self.increase_function_id();
                            return SmResult::EventHandled;
                        } else if *c == Key::PageDown {
                                self.decrease_function_id();
                                return SmResult::EventHandled;
                        } else {
                            match ParamSelector::select_item(&mut self.value_func_selection, *c) {
                                RetCode::KeyConsumed   => SmResult::EventHandled, // Selection updated
                                RetCode::KeyMissmatch  => SmResult::EventHandled, // Ignore key that doesn't match a selection
                                RetCode::ValueUpdated  => SmResult::EventHandled, // Selection updated
                                RetCode::Cancel        => SmResult::ChangeState(ParamSelector::state_parameter), // Stop updating the value, back to parameter selection
                                RetCode::ValueComplete => SmResult::ChangeState(ParamSelector::state_value_function_index), // Function selected
                                RetCode::Reset         => self.reset(),
                            }
                        }
                    }
                    SelectorEvent::ControlChange(ctrl, value) => SmResult::ChangeState(ParamSelector::state_value_function_index),
                    _ => SmResult::EventHandled,
                }
            }
        }
    }

    // Select which item in the function group to edit (Oscillator 1, 2, 3, ...)
    fn state_value_function_index(self: &mut ParamSelector,
                                  e: &SmEvent<SelectorEvent>)
    -> SmResult<ParamSelector, SelectorEvent> {
        match e {
            SmEvent::EnterState => {
                info!("state_value_function_index Enter");
                self.state = SelectorState::ValueFunctionIndex;
                SmResult::EventHandled
            }
            SmEvent::ExitState => {
                if self.value_changed {
                    match self.param_selection.value {
                        ParameterValue::Function(ref mut id) => {
                            info!("Saving function value");
                            id.function = self.value_func_selection.item_list[self.value_func_selection.item_index].item;
                            id.function_id = if let ParameterValue::Int(x) = self.value_func_selection.value { x as usize } else { panic!() };
                        },
                        _ => panic!(),
                    }
                }
                self.value_func_selection.temp_string.clear();
                SmResult::EventHandled
            }
            SmEvent::Event(selector_event) => {
                match selector_event {
                    SelectorEvent::Key(c) => {
                        if *c == Key::PageUp {
                            self.increase_function_id();
                            return SmResult::EventHandled;
                        } else if *c == Key::PageDown {
                                self.decrease_function_id();
                                return SmResult::EventHandled;
                        } else {
                            match ParamSelector::get_value(&mut self.value_func_selection, *c) {
                                RetCode::KeyConsumed   => SmResult::EventHandled, // Key has been used, but value hasn't changed
                                RetCode::KeyMissmatch  => SmResult::EventHandled, // Ignore unmatched keys
                                RetCode::ValueUpdated  => SmResult::EventHandled, // Selection not complete yet
                                RetCode::ValueComplete => {                       // Parameter has been selected
                                    // For modulation source or target, we might be finished here with
                                    // getting the value. Compare current state to expected target state.
                                    match self.param_selection.value {
                                        ParameterValue::Function(id) => {
                                            // Value is finished
                                            self.value_changed = true;
                                            SmResult::ChangeState(ParamSelector::state_parameter)
                                        },
                                        ParameterValue::Param(id) => {
                                            self.value_param_selection.item_list = self.value_func_selection.item_list[self.value_func_selection.item_index].next;

                                            // If function is same as before, set
                                            // value_param_selection.item_index to current value,
                                            // else set to valid value.
                                            self.query_current_value_param_index();

                                            SmResult::ChangeState(ParamSelector::state_value_parameter)
                                        },
                                        _ => SmResult::EventHandled,
                                    }
                                },
                                RetCode::Cancel        => SmResult::ChangeState(ParamSelector::state_value_parameter), // Abort function index selection
                                RetCode::Reset         => self.reset(),
                            }
                        }
                    }
                    SelectorEvent::ControlChange(ctrl, value) => {
                        ParamSelector::set_control_value(&mut self.value_func_selection, *value);
                        SmResult::EventHandled
                    }
                    _ => SmResult::EventHandled,
                }
            }
        }
    }

    // Select the parameter of the function to edit (Waveshape, Frequency, ...)
    fn state_value_parameter(self: &mut ParamSelector,
                             e: &SmEvent<SelectorEvent>)
    -> SmResult<ParamSelector, SelectorEvent> {
        match e {
            SmEvent::EnterState => {
                info!("state_value_parameter Enter");
                self.state = SelectorState::ValueParam;
                SmResult::EventHandled
            }
            SmEvent::ExitState => {
                if self.value_changed {
                    match self.param_selection.value {
                        ParameterValue::Param(ref mut id) => {
                            id.function = self.value_func_selection.item_list[self.value_func_selection.item_index].item;
                            id.function_id = if let ParameterValue::Int(x) = self.value_func_selection.value { x as usize } else { panic!() };
                            id.parameter = self.value_param_selection.item_list[self.value_param_selection.item_index].item;
                            info!("Saving parameter value {:?}", id);
                        },
                        _ => panic!(),
                    }
                }
                SmResult::EventHandled
            }
            SmEvent::Event(selector_event) => {
                match selector_event {
                    SelectorEvent::Key(c) => {
                        if *c == Key::PageUp {
                            self.increase_function_id();
                            return SmResult::EventHandled;
                        } else if *c == Key::PageDown {
                                self.decrease_function_id();
                                return SmResult::EventHandled;
                        } else {
                            match ParamSelector::select_item(&mut self.value_param_selection, *c) {
                                RetCode::KeyConsumed   => SmResult::EventHandled, // Value has changed, but not complete yet
                                RetCode::KeyMissmatch  => SmResult::EventHandled, // Ignore invalid key
                                RetCode::ValueUpdated  => {                       // Pararmeter selection updated
                                    SmResult::EventHandled
                                },
                                RetCode::ValueComplete => {                       // Prepare to read the value
                                    self.value_changed = true;
                                    SmResult::ChangeState(ParamSelector::state_parameter)
                                },
                                RetCode::Cancel        => SmResult::ChangeState(ParamSelector::state_value_function_index), // Abort function index selection
                                RetCode::Reset         => self.reset(),
                            }
                        }
                    }
                    SelectorEvent::ControlChange(ctrl, value) => {
                        SmResult::EventHandled
                    }
                    _ => SmResult::EventHandled,
                }
            }
        }
    }

    /* Select one of the items of functions or parameters.
     *
     * Called when a new user input is received and we're in the right state for function selection.
     * Updates the item_index of the ItemSelection.
     */
    fn select_item(item: &mut ItemSelection, c: termion::event::Key) -> RetCode {
        let result = match c {
            Key::Up => {
                if item.item_index < item.item_list.len() - 1 {
                    item.item_index += 1;
                }
                RetCode::ValueUpdated
            },
            Key::Down => {
                if item.item_index > 0 {
                    item.item_index -= 1;
                }
                RetCode::ValueUpdated
            },
            Key::Left | Key::Backspace => RetCode::Cancel,
            Key::Right => RetCode::ValueComplete,
            Key::Esc => RetCode::Reset,
            Key::Char('\n') => RetCode::ValueComplete,
            Key::Char(c) => {
                for (count, f) in item.item_list.iter().enumerate() {
                    if f.key == c {
                        item.item_index = count;
                        return RetCode::ValueComplete;
                    }
                }
                RetCode::KeyConsumed
            },
            _ => RetCode::KeyConsumed
        };
        info!("select_item {:?} (index {})", item.item_list[item.item_index].item, item.item_index);
        result
    }

    /* Construct the value of the selected item.
     *
     * Supports entering the value by
     * - Direct ascii input of the number
     * - Adjusting current value with Up or Down keys
     */
    fn get_value(item: &mut ItemSelection, c: termion::event::Key) -> RetCode {
        info!("get_value {:?}", item.item_list[item.item_index].item);

        match c {
            // Common keys
            Key::Esc   => RetCode::Reset,

            // All others
            _ => {
                match item.item_list[item.item_index].val_range {
                    ValueRange::Int(min, max) => ParamSelector::get_value_int(item, min, max, c),
                    ValueRange::Float(min, max, _) => ParamSelector::get_value_float(item, min, max, c),
                    ValueRange::Choice(choice_list) => ParamSelector::get_value_choice(item, choice_list, c),
                    _ => panic!(),
                }
            }
        }
    }

    /* Evaluate the MIDI control change message (ModWheel) */
    fn set_control_value(item: &mut ItemSelection, val: u64) {
        let value = item.item_list[item.item_index].val_range.translate_value(val);
        ParamSelector::update_value(item, value);
    }

    fn get_value_int(item: &mut ItemSelection,
                     min: i64,
                     max: i64,
                     c: termion::event::Key) -> RetCode {
        let mut current = if let ParameterValue::Int(x) = item.value { x } else { panic!() };
        let result = match c {
            Key::Char(x) => {
                match x {
                    '0' | '1' | '2' | '3' | '4' | '5' | '6' | '7' | '8' | '9' | '.' => {
                        let y = x as i64 - '0' as i64;
                        if item.temp_string.len() > 0 {
                            // Something already in temp string, append to end if possible.
                            let current_temp: Result<i64, ParseIntError> = item.temp_string.parse();
                            let current_temp = if let Ok(x) = current_temp {
                                x
                            } else {
                                item.temp_string.clear();
                                0
                            };
                            let val_digit_added = current_temp * 10 + y;
                            if val_digit_added > max {
                                // Value would be too big, ignore key
                                RetCode::KeyConsumed
                            } else {
                                item.temp_string.push(x);
                                let value: Result<i64, ParseIntError> = item.temp_string.parse();
                                current = if let Ok(x) = value { x } else { current };
                                if current * 10 > max {
                                    // No more digits can be added: Finished.
                                    RetCode::ValueComplete
                                } else {
                                    // Wait for more digits
                                    RetCode::ValueUpdated
                                }
                            }
                        } else {
                            if y * 10 < max {
                                // More digits could come, start temp_string
                                item.temp_string.push(x);
                                let value: Result<i64, ParseIntError> = item.temp_string.parse();
                                current = if let Ok(x) = value { x } else { current };
                                RetCode::ValueUpdated
                            } else {
                                current = y;
                                RetCode::ValueComplete
                            }
                        }
                    },
                    '+' => { current += 1; RetCode::ValueUpdated },
                    '-' => if current > min { current -= 1; RetCode::ValueUpdated } else { RetCode::KeyConsumed },
                    '\n' => RetCode::ValueComplete,
                    _ => RetCode::KeyMissmatch,
                }
            }
            Key::Up        => { current += 1; RetCode::ValueUpdated },
            Key::Down      => if current > min { current -= 1; RetCode::ValueUpdated } else { RetCode::KeyConsumed },
            Key::Left      => RetCode::Cancel,
            Key::Right     => RetCode::ValueComplete,
            Key::Backspace => RetCode::Cancel,
            _              => RetCode::ValueComplete,
        };
        match result {
            RetCode::ValueUpdated | RetCode::ValueComplete => ParamSelector::update_value(item, ParameterValue::Int(current)),
            _ => (),
        }
        result
    }

    fn get_value_float(item: &mut ItemSelection,
                       min: Float,
                       max: Float,
                       c: termion::event::Key) -> RetCode {
        let mut current = if let ParameterValue::Float(x) = item.value { x } else { panic!() };
        let result = match c {
            Key::Char(x) => {
                match x {
                    '0' | '1' | '2' | '3' | '4' | '5' | '6' | '7' | '8' | '9' | '.' => {
                        info!("Adding char {}", x);
                        item.temp_string.push(x);
                        let value: Result<Float, ParseFloatError> = item.temp_string.parse();
                        current = if let Ok(x) = value { x } else { current };
                        RetCode::ValueUpdated
                    },
                    '+' => { current += 1.0; RetCode::ValueUpdated },
                    '-' => { current -= 1.0; RetCode::ValueUpdated },
                    '\n' => RetCode::ValueComplete,
                    _ => RetCode::KeyMissmatch,
                }
            }
            // TODO: Use ValueRange 'step' value for inc/ dec, some for +/ - above
            Key::Up        => { current += 1.0; RetCode::ValueUpdated },
            Key::Down      => { current -= 1.0; RetCode::ValueUpdated },
            Key::Left      => RetCode::Cancel,
            Key::Right     => RetCode::ValueComplete,
            Key::Backspace => {
                let len = item.temp_string.len();
                if len > 0 {
                    item.temp_string.pop();
                    if item.temp_string.len() >= 1 {
                        let value = item.temp_string.parse();
                        current = if let Ok(x) = value { x } else { current };
                    } else {
                        item.temp_string.push('0');
                        current = 0.0;
                    }
                } else {
                    item.temp_string.push('0');
                    current = 0.0;
                }
                info!("BS for float value, remaining: {}, current = {}", item.temp_string, current);
                RetCode::ValueUpdated
            },
            _ => RetCode::KeyMissmatch,
        };
        match result {
            RetCode::ValueUpdated | RetCode::ValueComplete => ParamSelector::update_value(item, ParameterValue::Float(current)),
            _ => (),
        }
        result
    }

    fn get_value_choice(item: &mut ItemSelection,
                        choice_list: &'static [MenuItem],
                        c: termion::event::Key) -> RetCode {
        let mut current = if let ParameterValue::Choice(x) = item.value { x } else { panic!() };
        let result = match c {
            Key::Char('+') |
            Key::Up         => {current += 1; RetCode::ValueUpdated },
            Key::Char('-') |
            Key::Down       => if current > 0 { current -= 1; RetCode::ValueUpdated } else { RetCode::KeyConsumed },
            Key::Left       => RetCode::Cancel,
            Key::Right      => RetCode::ValueComplete,
            Key::Backspace  => RetCode::Cancel,
            Key::Char('\n') => RetCode::ValueComplete,
            _ => RetCode::KeyMissmatch,
        };
        match result {
            RetCode::ValueUpdated | RetCode::ValueComplete => ParamSelector::update_value(item, ParameterValue::Choice(current)),
            _ => (),
        }
        result
    }

    fn get_list_index(list: &[MenuItem], param: Parameter) -> usize {
        for (id, item) in list.iter().enumerate() {
            if item.item == param {
                return id;
            }
        }
        panic!("Didn't find parameter {:?} in list {:?}", param, list);
    }

    fn get_param_id_internal(fs: &ItemSelection, ps: &ItemSelection) -> ParamId {
        let function = &fs.item_list[fs.item_index];
        let function_id = if let ParameterValue::Int(x) = &fs.value { *x as usize } else { panic!() };
        let parameter = &ps.item_list[ps.item_index];
        ParamId::new(function.item, function_id, parameter.item)
    }

    pub fn get_param_id(&self) -> ParamId {
        ParamSelector::get_param_id_internal(&self.func_selection, &self.param_selection)
    }

    pub fn get_synth_param(&self) -> SynthParam {
        let function = &self.func_selection.item_list[self.func_selection.item_index];
        let function_id = if let ParameterValue::Int(x) = &self.func_selection.value { *x as usize } else { panic!() };
        let parameter = &self.param_selection.item_list[self.param_selection.item_index];
        SynthParam::new(function.item, function_id, parameter.item, self.param_selection.value)
    }

    /* Get value range for currently selected parameter. */
    pub fn get_value_range(&self) -> &'static ValueRange {
        &self.param_selection.item_list[self.param_selection.item_index].val_range
    }

    /* Select the parameter chosen by input, set value to current sound data. */
    fn query_current_value(&mut self) {
        let param = self.get_param_id();
        info!("query_current_value {:?}", param);

        // The value in the selected parameter needs to point to the right type.
        let sound = if let Some(sound) = &self.sound { sound } else { panic!() };
        let value = sound.borrow_mut().data.get_value(&param);
        info!("Sound has value {:?}", value);
        let next_item_list = &self.param_selection.item_list[self.param_selection.item_index].next;
        match value {
            // Let the two ItemSelection structs point to the currently active
            // values.
            ParameterValue::Function(id) => {
                let fs = &mut self.value_func_selection;
                fs.item_list = next_item_list;
                fs.item_index = ParamSelector::get_list_index(fs.item_list, id.function);
                fs.value = ParameterValue::Int(id.function_id as i64);
            }
            ParameterValue::Param(id) => {
                let fs = &mut self.value_func_selection;
                fs.item_list = next_item_list;
                fs.item_index = ParamSelector::get_list_index(fs.item_list, id.function);
                fs.value = ParameterValue::Int(id.function_id as i64);
                let ps = &mut self.value_param_selection;
                ps.item_index = ParamSelector::get_list_index(ps.item_list, id.parameter);
            }
            ParameterValue::NoValue => panic!(),
            _ => ()
        }
        self.param_selection.value = value;
    }

    /* Select the parameter chosen by input, set value to current sound data. */
    fn query_current_value_param_index(&mut self) {
        // Get current sound entry
        // Compare function
        // If equal, set same param Index, else set to 0

        let param = self.get_param_id();
        info!("query_current_value_param_index {:?}", param);

        // The value in the selected parameter needs to point to the right type.
        let sound = if let Some(sound) = &self.sound { sound } else { panic!() };
        let value = sound.borrow_mut().data.get_value(&param);
        info!("Sound has value {:?}", value);

        match value {
            ParameterValue::Param(id) => {
                let fs = &self.value_func_selection;
                let ps = &mut self.value_param_selection;
                let function = &fs.item_list[fs.item_index];
                if function.item == id.function {
                    info!("Same function, keep parameter index");
                    ps.item_index = ParamSelector::get_list_index(ps.item_list, id.parameter);
                } else {
                    info!("Function differs, set parameter index to safe value");
                    ps.item_index = 1;
                }
            }
            _ => ()
        }
    }

    /* Store a new value in the selected parameter. */
    fn update_value(selection: &mut ItemSelection, val: ParameterValue) {
        info!("update_value item: {:?}, value: {:?}", selection.item_list[selection.item_index].item, val);
        let temp_string = &mut selection.temp_string;
        match selection.item_list[selection.item_index].val_range {
            ValueRange::Int(min, max) => {
                let mut value = if let ParameterValue::Int(x) = val { x } else { panic!(); };
                if value > max {
                    value = max;
                }
                if value < min {
                    value = min;
                }
                selection.value = ParameterValue::Int(value);
            }
            ValueRange::Float(min, max, _) => {
                let mut value = if let ParameterValue::Float(x) = val { x } else { panic!(); };
                let has_period =  temp_string.contains(".");
                if value > max {
                    value = max;
                }
                if value < min {
                    value = min;
                }
                temp_string.clear();
                temp_string.push_str(value.to_string().as_str());
                if !temp_string.contains(".") && has_period {
                    temp_string.push('.');
                }
                selection.value = ParameterValue::Float(value);
            }
            ValueRange::Choice(selection_list) => {
                let mut value = if let ParameterValue::Choice(x) = val { x as usize } else { panic!("{:?}", val); };
                if value >= selection_list.len() {
                    value = selection_list.len() - 1;
                }
                selection.value = ParameterValue::Choice(value);
            }
            ValueRange::Dynamic(id) => {
                selection.value = val;
            }
            ValueRange::Func(selection_list) => {
                //panic!();
            }
            ValueRange::Param(selection_list) => {
                //panic!();
            }
            ValueRange::NoRange => {}
        };
    }

    /** A value has changed outside of the selector, update the local value. */
    pub fn value_has_changed(&mut self, sm: &mut StateMachine<ParamSelector, SelectorEvent>, param: SynthParam) {
        sm.handle_event(self, &SmEvent::Event(SelectorEvent::ValueChange(param)));
    }
}

// ----------------------------------------------
//                  Unit tests
// ----------------------------------------------

struct TestContext {
    ps: ParamSelector,
    sound: Rc<RefCell<SoundPatch>>,
    sm: StateMachine<ParamSelector, SelectorEvent>,
}

enum TestInput {
    Chars(String),
    Key(Key),
    ControlChange(u64, u64),
}

use flexi_logger::{Logger, opt_format};
static mut LOGGER_INITIALIZED: bool = false;

impl TestContext {

    fn new() -> TestContext {
        // Setup logging if required
        unsafe {
            if LOGGER_INITIALIZED == false {
                Logger::with_env_or_str("myprog=debug, mylib=warn")
                                        .log_to_file()
                                        .directory("log_files")
                                        .format(opt_format)
                                        .start()
                                        .unwrap();
                LOGGER_INITIALIZED = true;
            }
        }

        let ps = ParamSelector::new(&FUNCTIONS, &MOD_TARGETS);
        let sound = Rc::new(RefCell::new(SoundPatch::new()));
        let sm = StateMachine::new(ParamSelector::state_function);
        TestContext{ps, sound, sm}
    }

    /* Return 
     * - true if a value has changed and can be sent to the synth engine
     * - false if no value has
     */
    fn do_handle_input(&mut self, input: &TestInput) -> bool {
        let mut result = false;
        match input {
            TestInput::Chars(chars) => {
                for c in chars.chars() {
                    let k = Key::Char(c);
                    result = ParamSelector::handle_user_input(&mut self.ps, &mut self.sm, k, self.sound.clone())
                }
            }
            TestInput::Key(k) => {
                result = ParamSelector::handle_user_input(&mut self.ps, &mut self.sm, *k, self.sound.clone())
            }
            TestInput::ControlChange(controller, value) => {
                result = ParamSelector::handle_control_input(&mut self.ps, &mut self.sm, *controller, *value, self.sound.clone())
            }
        }
        result
    }

    /* Return 
     * - true if a value has changed and can be sent to the synth engine
     * - false if no value has
     */
    fn handle_input(&mut self, input: TestInput) -> bool {
        self.do_handle_input(&input)
    }

    /* Return 
     * - true if a value has changed and can be sent to the synth engine
     * - false if no value has
     */
    fn handle_inputs(&mut self, input: &[TestInput]) -> bool {
        let mut result = false;
        for i in input {
            result = self.do_handle_input(i);
        }
        result
    }

    fn verify_function(&self, func: Parameter) -> bool {
        let fs = &self.ps.func_selection;
        let function = fs.item_list[fs.item_index].item;
        if function != func {
            info!("Expected function {}, found {}", func, function);
            false
        } else {
            true
        }
    }

    fn verify_function_id(&self, func_id: usize) -> bool {
        let fs = &self.ps.func_selection;
        let function_id = if let ParameterValue::Int(x) = &fs.value { *x as usize } else { panic!() };
        if function_id != func_id {
            info!("Expected function ID {}, found {}", func_id, function_id);
            false
        } else {
            true
        }
    }

    fn verify_parameter(&self, param: Parameter) -> bool {
        let ps = &self.ps.param_selection;
        let parameter = ps.item_list[ps.item_index].item;
        if parameter != param {
            info!("Expected parameter {}, found {}", param, parameter);
            false
        } else {
            true
        }
    }

    fn verify_value(&self, value: ParameterValue) -> bool {
        let ps = &self.ps.param_selection;
        info!("Checking value {:?}", ps.value);
        match value {
            ParameterValue::Int(expected) => {
                let actual = if let ParameterValue::Int(j) = ps.value { j } else { panic!() };
                if expected != actual {
                    info!("Expected value {}, actual {}", expected, actual);
                    false
                } else {
                    true
                }
            }
            ParameterValue::Float(expected) => {
                let actual = if let ParameterValue::Float(j) = ps.value { j } else { panic!() };
                if expected != actual {
                    info!("Expected value {}, actual {}", expected, actual);
                    false
                } else {
                    true
                }
            },
            ParameterValue::Choice(expected) => {
                let actual = if let ParameterValue::Choice(j) = ps.value { j } else { panic!() };
                if expected != actual {
                    info!("Expected value {}, actual {}", expected, actual);
                    false
                } else {
                    true
                }
            },
            ParameterValue::Function(expected) => {
                info!("Expected value {:?}", expected);
                let actual = if let ParameterValue::Function(j) = ps.value { j } else { panic!() };
                if expected.function != actual.function
                || expected.function_id != actual.function_id {
                    info!("Expected value {:?}, actual {:?}", expected, actual);
                    false
                } else {
                    true
                }
            }
            ParameterValue::Param(expected) => {
                let actual = if let ParameterValue::Param(j) = ps.value { j } else { panic!() };
                if expected.function != actual.function
                || expected.function_id != actual.function_id {
                    info!("Expected value {:?}, actual {:?}", expected, actual);
                    false
                } else {
                    true
                }
            }
            _ => panic!(),
        }
    }

    fn verify_selection(&self,
                        func: Parameter,
                        func_id: usize,
                        param: Parameter,
                        value: ParameterValue) -> bool {
        self.verify_function(func) &&
        self.verify_function_id(func_id) &&
        self.verify_parameter(param) &&
        self.verify_value(value)
    }
}

// -----------------------
// Test function selection
// -----------------------

#[test]
fn test_direct_shortcuts_select_parameter() {
    let mut context = TestContext::new();

    // Initial state: Osc 1 something
    assert!(context.verify_selection(Parameter::Oscillator, 1, Parameter::Level, ParameterValue::Float(0.0)));
    assert_eq!(context.ps.state, SelectorState::Function);

    // Change to envelope 2 Sustain selection
    assert_eq!(context.handle_input(TestInput::Chars("e".to_string())), false);
    assert!(context.verify_function(Parameter::Envelope));
    assert_eq!(context.ps.state, SelectorState::FunctionIndex);
    assert_eq!(context.handle_input(TestInput::Chars("2".to_string())), false);
    assert!(context.verify_function_id(2));
    assert_eq!(context.ps.state, SelectorState::Param);
    assert_eq!(context.handle_input(TestInput::Chars("s".to_string())), false);
    assert!(context.verify_parameter(Parameter::Sustain));
    assert_eq!(context.ps.state, SelectorState::Value);
}

#[test]
fn test_invalid_shortcut_doesnt_change_function() {
    let mut context = TestContext::new();
    assert!(context.verify_selection(Parameter::Oscillator, 1, Parameter::Level, ParameterValue::Float(0.0)));
    assert_eq!(context.handle_input(TestInput::Chars("@".to_string())), false);
    assert!(context.verify_selection(Parameter::Oscillator, 1, Parameter::Level, ParameterValue::Float(0.0)));
    assert_eq!(context.ps.state, SelectorState::Function);
}

#[test]
fn test_cursor_navigation() {
    let mut context = TestContext::new();
    assert_eq!(context.ps.state, SelectorState::Function);

    // Forwards
    assert_eq!(context.handle_input(TestInput::Key(Key::Up)), false);
    assert!(context.verify_function(Parameter::Envelope));
    assert_eq!(context.handle_input(TestInput::Key(Key::Right)), false);
    assert_eq!(context.ps.state, SelectorState::FunctionIndex);
    assert!(context.verify_function_id(1));
    assert_eq!(context.handle_input(TestInput::Key(Key::Up)), false);
    assert!(context.verify_function_id(2));
    assert_eq!(context.handle_input(TestInput::Key(Key::Right)), false);
    assert_eq!(context.ps.state, SelectorState::Param);
    assert!(context.verify_parameter(Parameter::Attack));
    assert_eq!(context.handle_input(TestInput::Key(Key::Up)), false);
    assert!(context.verify_parameter(Parameter::Decay));
    assert_eq!(context.handle_input(TestInput::Key(Key::Right)), false);
    assert_eq!(context.ps.state, SelectorState::Value);
    assert!(context.verify_value(ParameterValue::Float(50.0)));

    // Backwards
    assert_eq!(context.handle_input(TestInput::Key(Key::Left)), false);
    assert_eq!(context.ps.state, SelectorState::Param);
    assert_eq!(context.handle_input(TestInput::Key(Key::Up)), false);
    assert!(context.verify_parameter(Parameter::Sustain));
    assert_eq!(context.handle_input(TestInput::Key(Key::Left)), false);
    assert_eq!(context.ps.state, SelectorState::FunctionIndex);
    assert_eq!(context.handle_input(TestInput::Key(Key::Down)), false);
    assert!(context.verify_function_id(1));
    assert_eq!(context.handle_input(TestInput::Key(Key::Left)), false);
    assert_eq!(context.ps.state, SelectorState::Function);
    assert_eq!(context.handle_input(TestInput::Key(Key::Up)), false);
    assert!(context.verify_function(Parameter::Lfo));
}

#[test]
fn test_param_selection_reads_current_value() {
    let mut context = TestContext::new();
    assert!(context.verify_selection(Parameter::Oscillator, 1, Parameter::Level, ParameterValue::Float(0.0)));

    // Change to level selection, reads current value from sound data
    assert_eq!(context.handle_input(TestInput::Chars("o1l".to_string())), false);
    assert!(context.verify_selection(Parameter::Oscillator, 1, Parameter::Level, ParameterValue::Float(92.0)));
}

#[test]
fn test_function_id_can_be_entered_directly() {
    let mut context = TestContext::new();
    context.handle_input(TestInput::Chars("o2v".to_string()));
    assert!(context.verify_selection(Parameter::Oscillator, 2, Parameter::Voices, ParameterValue::Int(1)));

    let c: &[TestInput] = &[TestInput::Key(Key::Esc), TestInput::Chars("m2a".to_string())];
    assert_eq!(context.handle_inputs(c), false);
    assert!(context.verify_selection(Parameter::Modulation, 2, Parameter::Amount, ParameterValue::Float(0.0)));

    let c: &[TestInput] = &[TestInput::Key(Key::Esc), TestInput::Chars("m12a".to_string())];
    assert_eq!(context.handle_inputs(c), false);
    assert!(context.verify_selection(Parameter::Modulation, 12, Parameter::Amount, ParameterValue::Float(0.0)));

    let c: &[TestInput] = &[TestInput::Key(Key::Esc), TestInput::Chars("m2a".to_string())];
    assert_eq!(context.handle_inputs(c), false);
    assert!(context.verify_selection(Parameter::Modulation, 2, Parameter::Amount, ParameterValue::Float(0.0)));
}

#[test]
fn test_cursor_up_increments_float_value() {
    let mut context = TestContext::new();
    let c: &[TestInput] = &[TestInput::Chars("o1l".to_string()), TestInput::Key(Key::Up)];
    assert_eq!(context.handle_inputs(c), true);
    assert!(context.verify_selection(Parameter::Oscillator, 1, Parameter::Level, ParameterValue::Float(93.0)));
}

#[test]
fn test_cursor_up_increments_int_value() {
    let mut context = TestContext::new();
    context.handle_input(TestInput::Chars("o1v".to_string()));
    assert!(context.verify_selection(Parameter::Oscillator, 1, Parameter::Voices, ParameterValue::Int(1)));
    assert_eq!(context.handle_input(TestInput::Key(Key::Up)), true);
    assert!(context.verify_selection(Parameter::Oscillator, 1, Parameter::Voices, ParameterValue::Int(2)));
}

#[test]
fn test_cursor_down_decrements_float_value() {
    let mut context = TestContext::new();
    context.handle_input(TestInput::Chars("o1l".to_string()));
    assert!(context.verify_selection(Parameter::Oscillator, 1, Parameter::Level, ParameterValue::Float(92.0)));
    assert_eq!(context.handle_input(TestInput::Key(Key::Down)), true);
    assert!(context.verify_selection(Parameter::Oscillator, 1, Parameter::Level, ParameterValue::Float(91.0)));
}

#[test]
fn test_cursor_down_decrements_int_value() {
    let mut context = TestContext::new();
    let c: &[TestInput] = &[TestInput::Chars("o1v".to_string()), TestInput::Key(Key::Up)];
    assert_eq!(context.handle_inputs(c), true);
    assert!(context.verify_selection(Parameter::Oscillator, 1, Parameter::Voices, ParameterValue::Int(2)));
    assert_eq!(context.handle_input(TestInput::Key(Key::Down)), true);
    assert!(context.verify_selection(Parameter::Oscillator, 1, Parameter::Voices, ParameterValue::Int(1)));
}

#[test]
fn test_multi_digit_index() {
    let mut context = TestContext::new();
    context.handle_input(TestInput::Chars("m".to_string()));
    assert!(context.verify_function(Parameter::Modulation));
    assert!(context.verify_function_id(1));
    context.handle_input(TestInput::Chars("12".to_string()));
    assert!(context.verify_function_id(12));
    assert_eq!(context.ps.state, SelectorState::Param);
}

#[test]
fn test_multi_digit_float_value() {
    let mut context = TestContext::new();
    context.handle_input(TestInput::Chars("o1l12.3456\n".to_string()));
    assert!(context.verify_selection(Parameter::Oscillator, 1, Parameter::Level, ParameterValue::Float(12.3456)));
}

#[test]
fn test_clear_int_tempstring_between_values() {

    // 1. After completing a value
    let mut context = TestContext::new();
    context.handle_input(TestInput::Chars("o1f23".to_string()));
    assert_eq!(context.ps.state, SelectorState::Param);
    assert!(context.verify_selection(Parameter::Oscillator, 1, Parameter::Frequency, ParameterValue::Int(23)));
    context.handle_input(TestInput::Chars("f21".to_string()));
    assert!(context.verify_selection(Parameter::Oscillator, 1, Parameter::Frequency, ParameterValue::Int(21)));

    // 2. After cancelling input
    let mut context = TestContext::new();
    let c: &[TestInput] = &[TestInput::Chars("o1f1".to_string()), TestInput::Key(Key::Backspace)];
    assert!(!context.handle_inputs(c));
    assert_eq!(context.ps.state, SelectorState::Param);
    assert!(context.verify_selection(Parameter::Oscillator, 1, Parameter::Frequency, ParameterValue::Int(1)));
    context.handle_input(TestInput::Chars("f23".to_string()));
    assert!(context.verify_selection(Parameter::Oscillator, 1, Parameter::Frequency, ParameterValue::Int(23)));
}

#[test]
fn test_escape_resets_to_valid_state() {
    // 1. Function state
    let mut context = TestContext::new();
    let c: &[TestInput] = &[TestInput::Key(Key::Up), TestInput::Key(Key::Esc)];
    assert_eq!(context.handle_inputs(c), false);
    assert!(context.verify_selection(Parameter::Oscillator, 1, Parameter::Level, ParameterValue::Float(0.0)));

    // 2. Function ID state
    let mut context = TestContext::new();
    let c: &[TestInput] = &[TestInput::Chars("o".to_string()), TestInput::Key(Key::Esc)];
    assert_eq!(context.handle_inputs(c), false);
    assert!(context.verify_selection(Parameter::Oscillator, 1, Parameter::Level, ParameterValue::Float(0.0)));

    // 3. Parameter state
    let mut context = TestContext::new();
    let c: &[TestInput] = &[TestInput::Chars("o1".to_string()), TestInput::Key(Key::Esc)];
    assert_eq!(context.handle_inputs(c), false);
    assert!(context.verify_selection(Parameter::Oscillator, 1, Parameter::Level, ParameterValue::Float(0.0)));

    // 4. Value state
    let mut context = TestContext::new();
    let c: &[TestInput] = &[TestInput::Chars("o1f".to_string()), TestInput::Key(Key::Esc)];
    assert_eq!(context.handle_inputs(c), false);
    assert!(context.verify_selection(Parameter::Oscillator, 1, Parameter::Level, ParameterValue::Float(0.0)));
}

#[test]
fn test_modulator_source_selection() {
    let mut context = TestContext::new();
    context.handle_input(TestInput::Chars("m\nsl1".to_string()));
    let value = FunctionId{function: Parameter::Lfo, function_id: 1};
    assert!(context.verify_selection(Parameter::Modulation, 1, Parameter::Source, ParameterValue::Function(value)));
    assert_eq!(context.ps.state, SelectorState::Param);
}

#[test]
fn test_modulator_target_selection() {
    let mut context = TestContext::new();
    let c: &[TestInput] = &[TestInput::Chars("m".to_string()),
                            TestInput::Key(Key::Right),
                            TestInput::Chars("td1t".to_string())];
    context.handle_inputs(c);
    let value = ParamId{function: Parameter::Delay, function_id: 1, parameter: Parameter::Time};
    assert!(context.verify_selection(Parameter::Modulation, 1, Parameter::Target, ParameterValue::Param(value)));
    assert_eq!(context.ps.state, SelectorState::Param);
}

#[test]
fn test_modulator_target_sel_with_cursor() {
    let mut context = TestContext::new();
    let c: &[TestInput] = &[TestInput::Chars("m".to_string()),
                            TestInput::Key(Key::Right), // Mod 1
                            TestInput::Key(Key::Up), // Target
                            TestInput::Key(Key::Right), // Target
                            TestInput::Key(Key::Right), // Oscillator
                            TestInput::Key(Key::Right), // Oscillator 1
                            ];
    context.handle_inputs(c);
    assert_eq!(context.ps.state, SelectorState::ValueParam);
}

#[test]
fn test_leave_subsel_with_cursor() {
    let mut context = TestContext::new();
    let c: &[TestInput] = &[TestInput::Chars("m".to_string()),
                            TestInput::Key(Key::Right), // Mod 1
                            TestInput::Key(Key::Right), // Source
                            ];
    context.handle_inputs(c);
    assert_eq!(context.ps.state, SelectorState::ValueFunction);
    context.handle_input(TestInput::Key(Key::Left)); // Back to parameter
    assert_eq!(context.ps.state, SelectorState::Param);
}

#[test]
fn test_controller_updates_selected_value() {
    let mut context = TestContext::new();
    let c: &[TestInput] = &[TestInput::Chars("o1l".to_string())];
    context.handle_inputs(c);
    assert_eq!(context.ps.state, SelectorState::Value);
    context.handle_input(TestInput::ControlChange(1, 0));
    assert_eq!(context.ps.state, SelectorState::Value);
    assert!(context.verify_selection(Parameter::Oscillator, 1, Parameter::Level, ParameterValue::Float(0.0)));
    context.handle_input(TestInput::ControlChange(1, 127));
    assert_eq!(context.ps.state, SelectorState::Value);
    assert!(context.verify_selection(Parameter::Oscillator, 1, Parameter::Level, ParameterValue::Float(100.0)));
}

#[test]
fn test_controller_is_ignored_in_other_states() {
    let mut context = TestContext::new();

    // Ignored in state_function
    context.handle_input(TestInput::ControlChange(1, 127));
    assert_eq!(context.ps.state, SelectorState::Function);

    // Ignored in state_function_id
    context.handle_input(TestInput::Chars("o".to_string()));
    context.handle_input(TestInput::ControlChange(1, 127));
    assert_eq!(context.ps.state, SelectorState::FunctionIndex);

    // In state_param, it switches to state_value
    context.handle_input(TestInput::Chars("1".to_string()));
    context.handle_input(TestInput::ControlChange(1, 127));
    assert_eq!(context.ps.state, SelectorState::Value);
}

#[test]
fn test_page_up_changes_function_id() {
    let mut context = TestContext::new();

    // Select Oscillator 1 Finetune
    let c: &[TestInput] = &[TestInput::Chars("o1t".to_string())];
    context.handle_inputs(c);
    assert_eq!(context.ps.state, SelectorState::Value);
    assert!(context.verify_selection(Parameter::Oscillator, 1, Parameter::Finetune, ParameterValue::Float(0.0)));

    // Enter PageUp
    context.handle_input(TestInput::Key(Key::PageUp));
    assert_eq!(context.ps.state, SelectorState::Value);
    assert!(context.verify_selection(Parameter::Oscillator, 2, Parameter::Finetune, ParameterValue::Float(0.0)));

    // Enter PageUp again
    context.handle_input(TestInput::Key(Key::PageUp));
    assert_eq!(context.ps.state, SelectorState::Value);
    assert!(context.verify_selection(Parameter::Oscillator, 3, Parameter::Finetune, ParameterValue::Float(0.0)));

    // Enter PageDown
    context.handle_input(TestInput::Key(Key::PageDown));
    assert_eq!(context.ps.state, SelectorState::Value);
    assert!(context.verify_selection(Parameter::Oscillator, 2, Parameter::Finetune, ParameterValue::Float(0.0)));
}

#[test]
fn test_float_string_input() {
    let mut context = TestContext::new();
    let c: &[TestInput] = &[TestInput::Chars("o3l".to_string()),
                            TestInput::Chars("13.5\n".to_string())];
    context.handle_inputs(c);
    assert_eq!(context.ps.state, SelectorState::Param);
    assert!(context.verify_selection(Parameter::Oscillator, 3, Parameter::Level, ParameterValue::Float(13.5)));

    let c: &[TestInput] = &[TestInput::Chars("l".to_string()),
                            TestInput::Chars("26.47".to_string()),
                            TestInput::Key(Key::Backspace),
                            TestInput::Chars("\n".to_string()),
    ];
    context.handle_inputs(c);
    assert_eq!(context.ps.state, SelectorState::Param);
    assert!(context.verify_selection(Parameter::Oscillator, 3, Parameter::Level, ParameterValue::Float(26.4)));
}

// TODO:
// - Select next param from value state with param shortcut
// - Delete controller assignment in MIDI learn mode

