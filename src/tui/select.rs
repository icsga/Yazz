use super::Float;
use super::{FunctionId, ParamId};
use super::{Parameter, ParameterValue, SynthParam, ValueRange, MenuItem, FUNCTIONS, OSC_PARAMS, MOD_SOURCES, MOD_TARGETS};
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
        Value => Param,
        ValueFunction => panic!(),
        ValueFunctionIndex => panic!(),
        ValueParam => panic!(),
    }
}

pub fn previous(current: SelectorState) -> SelectorState {
    use SelectorState::*;
    match current {
        Function => Function,
        FunctionIndex => Function,
        Param => FunctionIndex,
        Value => Param,
        ValueFunction => panic!(),
        ValueFunctionIndex => panic!(),
        ValueParam => panic!(),
    }
}

#[derive(Debug)]
pub struct ItemSelection {
    pub item_list: &'static [MenuItem], // The MenuItem this item is coming from
    pub item_index: usize,              // Index into the MenuItem list
    pub value: ParameterValue,          // ID or value of the selected item
    pub temp_string: String,
}

impl ItemSelection {
    pub fn reset(&mut self) {
        self.item_index = 0;
        self.value = match self.item_list[0].val_range {
            ValueRange::IntRange(min, _) => ParameterValue::Int(min),
            ValueRange::FloatRange(min, _) => ParameterValue::Float(min),
            ValueRange::ChoiceRange(_) => ParameterValue::Choice(0),
            ValueRange::FuncRange(func_list) => ParameterValue::Function(FunctionId{function: func_list[0].item, function_id: 0}),
            ValueRange::ParamRange(func_list) => ParameterValue::Function(FunctionId{function: func_list[0].item, function_id: 0}),
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

#[derive(Debug)]
pub struct ParamSelector {
    value_changed: bool,
    pub state: SelectorState,
    pub func_selection: ItemSelection,
    pub param_selection: ItemSelection,
    pub value: ParameterValue,

    // Used for modulation source/ target: When reaching this state, the value
    // is complete. For normal parameters, this will be Value, for modulation
    // parameters it is either FunctionIndex or Parameter.
    pub target_state: SelectorState,

    //pub temp_string: String,
    pub sub_selector: Option<Rc<RefCell<ParamSelector>>>,

    sound: Option<Rc<RefCell<SoundData>>>,
}

impl ParamSelector {
    pub fn new(function_list: &'static [MenuItem]) -> ParamSelector {
        let func_selection = ItemSelection{
            item_list: function_list,
            item_index: 0,
            value: ParameterValue::Int(1),
            temp_string: String::new()};
        let param_selection = ItemSelection{item_list: function_list[0].next,
            item_index: 0,
            value: ParameterValue::Int(1),
            temp_string: String::new()};
        ParamSelector{value_changed: false,
                      state: SelectorState::Function,
                      func_selection: func_selection,
                      param_selection: param_selection,
                      value: ParameterValue::Int(0),
                      target_state: SelectorState::Value,
                      //temp_string: String::new(),
                      sub_selector: Option::None,
                      sound: Option::None}
    }

    pub fn set_subselector(&mut self, subsel: ParamSelector) {
        self.sub_selector = Option::Some(Rc::new(RefCell::new(subsel)));
    }

    pub fn reset(&mut self) -> SmResult<ParamSelector, termion::event::Key> {
        self.func_selection.reset();
        self.param_selection.reset();
        SmResult::ChangeState(ParamSelector::state_function)
    }

    /* Received a keyboard event from the terminal.
     *
     * Return 
     * - true if value is complete and can be sent to the synth engine
     * - false if value is not complete yet
     */
    pub fn handle_user_input(&mut self,
                             sm: &mut StateMachine<ParamSelector, termion::event::Key>,
                             c: termion::event::Key,
                             sound: Rc<RefCell<SoundData>>) -> bool {
        info!("handle_user_input {:?} in state {:?}", c, self.state);
        self.sound = Option::Some(Rc::clone(&sound));
        sm.handle_event(self, &SmEvent::Event(c));
        self.value_changed
    }

    // Select the function group to edit (Oscillator, Envelope, ...)
    pub fn state_function(self: &mut ParamSelector,
                      e: &SmEvent<termion::event::Key>)
    -> SmResult<ParamSelector, termion::event::Key> {
        match e {
            SmEvent::EnterState => {
                info!("state_function Enter");
                self.state = SelectorState::Function;
                SmResult::EventHandled
            }
            SmEvent::ExitState => {
                info!("state_function Exit");
                SmResult::EventHandled
            }
            SmEvent::Event(c) => {
                match ParamSelector::select_item(&mut self.func_selection, *c) {
                    RetCode::KeyConsumed   => SmResult::EventHandled, // Selection updated
                    RetCode::KeyMissmatch  => SmResult::EventHandled, // Ignore key that doesn't match a selection
                    RetCode::ValueUpdated  => SmResult::EventHandled, // Selection updated
                    RetCode::Cancel        => SmResult::EventHandled,
                    RetCode::ValueComplete => SmResult::ChangeState(ParamSelector::state_function_index), // Function selected
                    RetCode::Reset         => self.reset(),
                }
            }
        }
    }

    // Select which item in the function group to edit (Oscillator 1, 2, 3, ...)
    fn state_function_index(self: &mut ParamSelector,
                            e: &SmEvent<termion::event::Key>)
    -> SmResult<ParamSelector, termion::event::Key> {
        match e {
            SmEvent::EnterState => {
                info!("state_function_index Enter");
                self.state = SelectorState::FunctionIndex;
                SmResult::EventHandled
            }
            SmEvent::ExitState => {
                info!("state_function_index Exit");
                SmResult::EventHandled
            }
            SmEvent::Event(c) => {
                match ParamSelector::get_value(&mut self.func_selection, *c) {
                    RetCode::KeyConsumed   => SmResult::EventHandled, // Key has been used, but value hasn't changed
                    RetCode::KeyMissmatch  => SmResult::EventHandled, // Ignore unmatched keys
                    RetCode::ValueUpdated  => SmResult::EventHandled, // Selection not complete yet
                    RetCode::ValueComplete => {                  // Parameter has been selected
                        self.param_selection.item_list = self.func_selection.item_list[self.func_selection.item_index].next;
                        self.select_param();
                        SmResult::ChangeState(ParamSelector::state_parameter)
                    },
                    RetCode::Cancel        => SmResult::ChangeState(ParamSelector::state_function), // Abort function index selection
                    RetCode::Reset         => self.reset(),
                }
            }
        }
    }

    // Select the parameter of the function to edit (Waveshape, Frequency, ...)
    fn state_parameter(self: &mut ParamSelector,
                       e: &SmEvent<termion::event::Key>)
    -> SmResult<ParamSelector, termion::event::Key> {
        match e {
            SmEvent::EnterState => {
                info!("state_parameter Enter");
                self.state = SelectorState::Param;
                SmResult::EventHandled
            }
            SmEvent::ExitState => {
                info!("state_parameter Exit");
                SmResult::EventHandled
            }
            SmEvent::Event(c) => {
                match ParamSelector::select_item(&mut self.param_selection, *c) {
                    RetCode::KeyConsumed   => SmResult::EventHandled, // Value has changed, but not complete yet
                    RetCode::KeyMissmatch  => SmResult::EventHandled, // Ignore invalid key
                    RetCode::ValueUpdated  => {                     // Pararmeter selection updated
                        self.select_param();
                        //SmResult::ChangeState(ParamSelector::state_value)
                        SmResult::EventHandled
                    },
                    RetCode::ValueComplete => {                     // Prepare to read the value
                        self.select_param();
                        SmResult::ChangeState(ParamSelector::state_value)
                    },
                    RetCode::Cancel        => SmResult::ChangeState(ParamSelector::state_function_index), // Cancel parameter selection
                    RetCode::Reset         => self.reset(),
                }
            }
        }
    }

    // Select the parameter value
    fn state_value(self: &mut ParamSelector,
                   e: &SmEvent<termion::event::Key>)
    -> SmResult<ParamSelector, termion::event::Key> {
        match e {
            SmEvent::EnterState => {
                info!("state_value Enter");
                self.state = SelectorState::Value;
                self.param_selection.temp_string.clear();
                SmResult::EventHandled
            }
            SmEvent::ExitState => {
                info!("state_value Exit");
                SmResult::EventHandled
            }
            SmEvent::Event(c) => {
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
        }
    }

    // Select the function group to edit (Oscillator, Envelope, ...)
    fn state_value_function(self: &mut ParamSelector,
                            e: &SmEvent<termion::event::Key>)
    -> SmResult<ParamSelector, termion::event::Key> {
        match e {
            SmEvent::EnterState => {
                info!("state_value_function Enter");
                self.state = SelectorState::ValueFunction;
                SmResult::EventHandled
            }
            SmEvent::ExitState => {
                info!("state_value_function Exit");
                SmResult::EventHandled
            }
            SmEvent::Event(c) => {
                match ParamSelector::select_item(&mut self.func_selection, *c) {
                    RetCode::KeyConsumed   => SmResult::EventHandled, // Selection updated
                    RetCode::KeyMissmatch  => SmResult::EventHandled, // Ignore key that doesn't match a selection
                    RetCode::ValueUpdated  => SmResult::EventHandled, // Selection updated
                    RetCode::Cancel        => SmResult::EventHandled,
                    RetCode::ValueComplete => SmResult::ChangeState(ParamSelector::state_value_function_index), // Function selected
                    RetCode::Reset         => self.reset(),
                }
            }
        }
    }

    // Select which item in the function group to edit (Oscillator 1, 2, 3, ...)
    fn state_value_function_index(self: &mut ParamSelector,
                                  e: &SmEvent<termion::event::Key>)
    -> SmResult<ParamSelector, termion::event::Key> {
        match e {
            SmEvent::EnterState => {
                info!("state_value_function_index Enter");
                self.state = SelectorState::ValueFunctionIndex;
                SmResult::EventHandled
            }
            SmEvent::ExitState => {
                info!("state_value_function_index Exit");
                SmResult::EventHandled
            }
            SmEvent::Event(c) => {
                match ParamSelector::get_value(&mut self.param_selection, *c) {
                    RetCode::KeyConsumed   => SmResult::EventHandled, // Key has been used, but value hasn't changed
                    RetCode::KeyMissmatch  => SmResult::EventHandled, // Ignore unmatched keys
                    RetCode::ValueUpdated  => SmResult::EventHandled, // Selection not complete yet
                    RetCode::ValueComplete => {                       // Parameter has been selected
                        // For modulation source or target, we might be finished here with
                        // getting the value. Compare current state to expected target state.
                        //
                        //
                        //
                        // TODO
                        //
                        //
                        /*
                        if self.state == self.target_state {
                            self.value_changed = true;
                            SmResult::ChangeState(ParamSelector::state_value_function)
                        } else {
                            self.param_selection.item_list = self.func_selection.item_list[self.func_selection.item_index].next;
                            self.select_param();
                            SmResult::ChangeState(ParamSelector::state_value_parameter)
                        }
                        */
                        SmResult::EventHandled
                    },
                    RetCode::Cancel        => SmResult::ChangeState(ParamSelector::state_value_parameter), // Abort function index selection
                    RetCode::Reset         => self.reset(),
                }
            }
        }
    }

    // Select the parameter of the function to edit (Waveshape, Frequency, ...)
    fn state_value_parameter(self: &mut ParamSelector,
                             e: &SmEvent<termion::event::Key>)
    -> SmResult<ParamSelector, termion::event::Key> {
        match e {
            SmEvent::EnterState => {
                info!("state_value_parameter Enter");
                self.state = SelectorState::ValueParam;
                SmResult::EventHandled
            }
            SmEvent::ExitState => {
                info!("state_value_parameter Exit");
                SmResult::EventHandled
            }
            SmEvent::Event(c) => {
                match ParamSelector::select_item(&mut self.param_selection, *c) {
                    RetCode::KeyConsumed   => SmResult::EventHandled, // Value has changed, but not complete yet
                    RetCode::KeyMissmatch  => SmResult::EventHandled, // Ignore invalid key
                    RetCode::ValueUpdated  => {                       // Pararmeter selection updated
                        self.select_param();
                        SmResult::EventHandled
                    },
                    RetCode::ValueComplete => {                       // Prepare to read the value
                        // For modulation source or target, we might be finished here with
                        // getting the value. Compare current state to expected target state.
                        //
                        //
                        //
                        //
                        // TODO
                        //
                        //
                        //
                        /*
                        if self.state == self.target_state {
                            self.value_changed = true;
                            SmResult::ChangeState(ParamSelector::state_value_function_index)
                        } else {
                            self.select_param();
                            SmResult::ChangeState(ParamSelector::state_parameter)
                        }
                        */
                        SmResult::EventHandled
                    },
                    RetCode::Cancel        => SmResult::ChangeState(ParamSelector::state_value_function_index), // Abort function index selection
                    RetCode::Reset         => self.reset(),
                }
            }
        }
    }

    /* Change the state of the input state machine. */
    /*
    fn change_state(&mut self, new_state: SelectorState) {
        if new_state != self.state {

            // Exiting current state, perform some cleanup
            match self.state {
                SelectorState::FunctionIndex => {
                    self.temp_string.clear();
                },
                SelectorState::Value => {
                    self.temp_string.clear();
                },
                _ => ()
            }

            // Entering next state, initialize needed data
            match new_state {
                SelectorState::Value => {
                    // If we're using the sub-selector, we need to set the right
                    // parameter list (e.g. mod source or mod target).
                    info!("Initializing sub-selector");
                    let subsel = if let Some(subsel) = &self.sub_selector { subsel } else { panic!() };
                    let mut subref = subsel.borrow_mut();
                    let ps = &self.param_selection;
                    match ps.item_list[ps.item_index].val_range {
                        ValueRange::FuncRange(list) => {
                            subref.func_selection.item_list = &MOD_SOURCES;
                            subref.target_state = SelectorState::FunctionIndex;
                            subref.state = SelectorState::Function;
                        },
                        ValueRange::ParamRange(list) => {
                            subref.func_selection.item_list = &MOD_TARGETS;
                            subref.target_state = SelectorState::Param;
                            subref.state = SelectorState::Function;
                        },
                        _ => (),
                    }
                }
                _ => ()
            }

            if let SelectorState::Function = new_state {
                self.param_selection.item_index = 0;
            }
            info!("change_state {} -> {}", self.state, new_state);
            self.state = new_state;
        }
    }
    */

    /* Select one of the items of functions or parameters.
     *
     * Called when a new user input is received and we're in the right state for function selection.
     */
    fn select_item(item: &mut ItemSelection, c: termion::event::Key) -> RetCode {
        info!("select_item");
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
        info!("select_item {:?}", item.item_list[item.item_index].item);
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
                    ValueRange::IntRange(min, max) => ParamSelector::get_value_int(item, min, max, c),
                    ValueRange::FloatRange(min, max) => ParamSelector::get_value_float(item, min, max, c),
                    ValueRange::ChoiceRange(choice_list) => ParamSelector::get_value_choice(item, choice_list, c),
                    ValueRange::FuncRange(_) | ValueRange::ParamRange(_) => {
                        // TODO: This
                        //ParamSelector::get_value_subselector(item, c),
                        RetCode::KeyConsumed
                    },
                    _ => panic!(),
                }
            }
        }
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
                    '\n' => RetCode::ValueComplete,
                    _ => RetCode::KeyMissmatch,
                }
            }
            Key::Up        => { current += 1.0; RetCode::ValueUpdated },
            Key::Down      => { current -= 1.0; RetCode::ValueUpdated },
            Key::Left      => RetCode::Cancel,
            Key::Right     => RetCode::ValueComplete,
            Key::Backspace => {
                let len = item.temp_string.len();
                if len > 0 {
                    item.temp_string.pop();
                    if len >= 1 {
                        let value = item.temp_string.parse();
                        current = if let Ok(x) = value { x } else { current };
                    } else {
                        current = 0.0;
                    }
                }
                RetCode::KeyConsumed
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
            Key::Up         => {current += 1; RetCode::ValueUpdated },
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

    fn get_value_subselector(item: &mut ItemSelection,
                             c: termion::event::Key) -> RetCode {
        /*
        // Pass key to sub selector
        let result = match &mut s.sub_selector {
            Some(sub) => {
                info!("Calling sub-selector!");
                ParamSelector::handle_user_input(&mut sub.borrow_mut(), c)
            },
            None => panic!(),
        };
        if result == true {
            info!("Subselector value complete");
            let selector = if let Some(selector) = &s.sub_selector {selector} else {panic!()};
            let selector = selector.borrow();
            info!("func_selection: {:?}", s.param_selection);
            info!("param_selection: {:?}", s.param_selection);
            match s.param_selection.value {
                ParameterValue::Function(ref mut v) => {
                    let s_f = &selector.func_selection;
                    v.function = s_f.item_list[s_f.item_index].item;
                    v.function_id = if let ParameterValue::Int(x) = s_f.value { x as usize } else { panic!() };
                    info!("Copying function {:?}", v);
                },
                ParameterValue::Param(ref mut v) => {
                    let s_f = &selector.func_selection;
                    v.function = s_f.item_list[s_f.item_index].item;
                    v.function_id = if let ParameterValue::Int(x) = s_f.value { x as usize } else { panic!() };
                    let s_p = &selector.param_selection;
                    v.parameter = s_p.item_list[s_p.item_index].item;
                    info!("Copying parameter {:?}", v);
                },
                _ => panic!(),
            }
            info!("Selector value after setting = {:?}", s.param_selection.value);
        }
        result
        */
        RetCode::KeyConsumed
    }
    
    fn get_list_index(list: &[MenuItem], param: Parameter) -> usize {
        for (id, item) in list.iter().enumerate() {
            if item.item == param {
                return id;
            }
        }
        panic!();
    }

    /* Select the parameter chosen by input, set value to current sound data. */
    fn select_param(&mut self) {
        info!("select_param {:?}", self.param_selection.item_list[self.param_selection.item_index].item);
        //let param = item.item_list[item.item_index].item;

        let function = &self.func_selection.item_list[self.func_selection.item_index];
        let function_id = if let ParameterValue::Int(x) = &self.func_selection.value { *x as usize } else { panic!() };
        let parameter = &self.param_selection.item_list[self.param_selection.item_index];
        let param_val = &self.param_selection.value;
        let param = SynthParam::new(function.item, function_id, parameter.item, *param_val);

        // The value in the selected parameter needs to point to the right type.
        let sound = if let Some(sound) = &self.sound { sound } else { panic!() };
        let value = sound.borrow_mut().get_value(&param);
        info!("Sound has value {:?}", value);
        match value {
            ParameterValue::Function(id) => {
                let fs = &mut self.sub_selector.as_ref().unwrap().borrow_mut().func_selection;
                fs.item_index = ParamSelector::get_list_index(fs.item_list, id.function);
                fs.value = ParameterValue::Int(id.function_id as i64);
            }
            ParameterValue::Param(id) => {
                let ss = &mut self.sub_selector.as_ref().unwrap().borrow_mut();
                ss.func_selection.item_index = ParamSelector::get_list_index(ss.func_selection.item_list, id.function);
                ss.func_selection.value = ParameterValue::Int(id.function_id as i64);
                ss.param_selection.item_index = ParamSelector::get_list_index(ss.param_selection.item_list, id.parameter);
                ss.param_selection.value = ParameterValue::Int(0);
            }
            ParameterValue::NoValue => panic!(),
            _ => ()
        }
        self.param_selection.value = value;
    }

    /* Store a new value in the selected parameter. */
    fn update_value(selection: &mut ItemSelection, val: ParameterValue) {
        info!("update_value item: {:?}, value: {:?}", selection.item_list[selection.item_index].item, val);
        let temp_string = &mut selection.temp_string;
        match selection.item_list[selection.item_index].val_range {
            ValueRange::IntRange(min, max) => {
                let mut val = if let ParameterValue::Int(x) = val { x } else { panic!(); };
                if val > max {
                    val = max;
                }
                if val < min {
                    val = min;
                }
                selection.value = ParameterValue::Int(val.try_into().unwrap());
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
                selection.value = ParameterValue::Float(val);
            }
            ValueRange::ChoiceRange(selection_list) => {
                let mut val = if let ParameterValue::Choice(x) = val { x as usize } else { panic!("{:?}", val); };
                if val >= selection_list.len() {
                    val = selection_list.len() - 1;
                }
                selection.value = ParameterValue::Choice(val);
            }
            ValueRange::FuncRange(selection_list) => {
                panic!();
            }
            ValueRange::ParamRange(selection_list) => {
                panic!();
            }
            ValueRange::NoRange => {}
        };
    }

    /* Evaluate the MIDI control change message (ModWheel) */
    pub fn handle_control_change(s: &mut ParamSelector, val: i64) {
        //
        //
        //
        //
        // TODO
        //
        //
        //
        //
        /*
        match s.state {
            SelectorState::Function => ParamSelector::change_state(s, SelectorState::FunctionIndex),
            SelectorState::FunctionIndex => (),
            SelectorState::Param => ParamSelector::change_state(s, SelectorState::Value),
            SelectorState::Value => (),
            SelectorState::ValueFunction => (),
            SelectorState::ValueFunctionIndex => (),
            SelectorState::ValueParam => (),
        }
        let item = &mut s.param_selection;
        match item.item_list[item.item_index].val_range {
            ValueRange::IntRange(min, max) => {
                let inc: Float = (max - min) as Float / 127.0;
                let value = min + (val as Float * inc) as i64;
                ParamSelector::update_value(item, ParameterValue::Int(value), &mut s.temp_string);
            }
            ValueRange::FloatRange(min, max) => {
                let inc: Float = (max - min) / 127.0;
                let value = min + val as Float * inc;
                ParamSelector::update_value(item, ParameterValue::Float(value), &mut s.temp_string);
            }
            ValueRange::ChoiceRange(choice_list) => {
                let inc: Float = choice_list.len() as Float / 127.0;
                let value = (val as Float * inc) as i64;
                ParamSelector::update_value(item, ParameterValue::Choice(value as usize), &mut s.temp_string);
            }
            _ => ()
        }
        */
    }
}


// ----------------------------------------------
//                  Unit tests
// ----------------------------------------------

struct TestContext {
    ps: ParamSelector,
    sound: SoundPatch,
    sound_data: Rc<RefCell<SoundData>>,
    sm: StateMachine<ParamSelector, termion::event::Key>,
}

enum TestInput {
    Chars(String),
    Key(Key)
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

        let sub = ParamSelector::new(&MOD_SOURCES);
        let mut ps = ParamSelector::new(&FUNCTIONS);
        ps.set_subselector(sub);
        let sound = SoundPatch::new();
        let sound_data = Rc::new(RefCell::new(sound.data));
        let sm = StateMachine::new(ParamSelector::state_function);
        TestContext{ps, sound, sound_data, sm}
    }

    fn do_handle_input(&mut self, input: &TestInput) -> bool {
        let mut result = false;
        match input {
            TestInput::Chars(chars) => {
                for c in chars.chars() {
                    let k = Key::Char(c);
                    result = ParamSelector::handle_user_input(&mut self.ps, &mut self.sm, k, self.sound_data.clone())
                }
            }
            TestInput::Key(k) => {
                result = ParamSelector::handle_user_input(&mut self.ps, &mut self.sm, *k, self.sound_data.clone())
            }
        }
        result
    }

    fn handle_input(&mut self, input: TestInput) -> bool {
        self.do_handle_input(&input)
    }

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

    // Initial state: Osc 1 waveform Sine
    assert!(context.verify_selection(Parameter::Oscillator, 1, Parameter::Waveform, ParameterValue::Int(1)));
    //assert_eq!(context.ps.state, SelectorState::Function);

    // Change to envelope 2 Sustain selection
    assert_eq!(context.handle_input(TestInput::Chars("e".to_string())), false);
    assert!(context.verify_function(Parameter::Envelope));
    //assert_eq!(context.ps.state, SelectorState::FunctionIndex);
    assert_eq!(context.handle_input(TestInput::Chars("2".to_string())), false);
    assert!(context.verify_function_id(2));
    //assert_eq!(context.ps.state, SelectorState::Param);
    assert_eq!(context.handle_input(TestInput::Chars("s".to_string())), false);
    assert!(context.verify_parameter(Parameter::Sustain));
    //assert_eq!(context.ps.state, SelectorState::Value);
}

#[test]
fn test_invalid_shortcut_doesnt_change_function() {
    let mut context = TestContext::new();
    assert!(context.verify_selection(Parameter::Oscillator, 1, Parameter::Waveform, ParameterValue::Int(1)));
    assert_eq!(context.handle_input(TestInput::Chars("@".to_string())), false);
    assert!(context.verify_selection(Parameter::Oscillator, 1, Parameter::Waveform, ParameterValue::Int(1)));
    assert_eq!(context.ps.state, SelectorState::Function);
}

#[test]
fn test_cursor_navigation() {
    let mut context = TestContext::new();
    assert!(context.verify_selection(Parameter::Oscillator, 1, Parameter::Waveform, ParameterValue::Int(1)));
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
    assert!(context.verify_selection(Parameter::Oscillator, 1, Parameter::Waveform, ParameterValue::Int(1)));

    // Change to level selection, reads current value from sound data
    assert_eq!(context.handle_input(TestInput::Chars("o1l".to_string())), false);
    assert!(context.verify_selection(Parameter::Oscillator, 1, Parameter::Level, ParameterValue::Float(92.0)));
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
    assert!(context.handle_inputs(c));
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
    assert!(context.verify_selection(Parameter::Oscillator, 1, Parameter::Waveform, ParameterValue::Choice(0)));

    // 2. Function ID state
    let mut context = TestContext::new();
    let c: &[TestInput] = &[TestInput::Chars("o".to_string()), TestInput::Key(Key::Esc)];
    assert_eq!(context.handle_inputs(c), false);
    assert!(context.verify_selection(Parameter::Oscillator, 1, Parameter::Waveform, ParameterValue::Choice(0)));

    // 3. Parameter state
    let mut context = TestContext::new();
    let c: &[TestInput] = &[TestInput::Chars("o1".to_string()), TestInput::Key(Key::Esc)];
    assert_eq!(context.handle_inputs(c), false);
    assert!(context.verify_selection(Parameter::Oscillator, 1, Parameter::Waveform, ParameterValue::Choice(0)));

    // 4. Value state
    let mut context = TestContext::new();
    let c: &[TestInput] = &[TestInput::Chars("o1f".to_string()), TestInput::Key(Key::Esc)];
    assert_eq!(context.handle_inputs(c), false);
    assert!(context.verify_selection(Parameter::Oscillator, 1, Parameter::Waveform, ParameterValue::Choice(0)));
}

#[test]
fn test_modulator_source_selection() {
    let mut context = TestContext::new();
    context.handle_input(TestInput::Chars("m\nsl1".to_string()));
    let value = FunctionId{function: Parameter::Lfo, function_id: 1};
    assert!(context.verify_selection(Parameter::Modulation, 1, Parameter::Source, ParameterValue::Function(value)));
    assert_eq!(context.ps.state, SelectorState::Param);
}

/*
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
    assert_eq!(context.ps.state, SelectorState::Value);
    assert_eq!(context.ps.sub_selector.as_ref().unwrap().borrow().state, SelectorState::Param);
}

#[test]
fn test_leave_subsel_with_cursor() {
    let mut context = TestContext::new();
    let c: &[TestInput] = &[TestInput::Chars("m".to_string()),
                            TestInput::Key(Key::Right), // Mod 1
                            TestInput::Key(Key::Right), // Source
                            ];
    context.handle_inputs(c);
    assert_eq!(context.ps.state, SelectorState::Value);
    assert_eq!(context.ps.sub_selector.as_ref().unwrap().borrow().state, SelectorState::Function);
    context.handle_input(TestInput::Key(Key::Left)); // Back to parameter
    assert_eq!(context.ps.state, SelectorState::Param);
}
*/
