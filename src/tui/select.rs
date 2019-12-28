use super::Float;
use super::{Parameter, ParameterValue, ParamId, FunctionId, SynthParam, ValueRange, MenuItem, FUNCTIONS, OSC_PARAMS, MOD_SOURCES, MOD_TARGETS};
use super::SoundData;

use log::{info, trace, warn};
use termion::event::Key;

use std::cell::RefCell;
use std::convert::TryInto;
use std::fmt::{self, Debug};
use std::num::ParseFloatError;
use std::rc::Rc;

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum SelectorState {
    Function,
    FunctionIndex,
    Param,
    Value,
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
    }
}

pub fn previous(current: SelectorState) -> SelectorState {
    use SelectorState::*;
    match current {
        Function => Function,
        FunctionIndex => Function,
        Param => FunctionIndex,
        Value => Param,
    }
}

#[derive(Debug)]
pub struct ItemSelection {
    pub item_list: &'static [MenuItem], // The MenuItem this item is coming from
    pub item_index: usize,              // Index into the MenuItem list
    pub value: ParameterValue,             // ID or value of the selected item
}

/* Return code from functions handling user input. */
#[derive(PartialEq)]
enum RetCode {
    KeyConsumed,   // Key has been used, but value is not updated yet
    ValueUpdated,  // Key has been used and value has changed to a valid value
    ValueComplete, // Value has changed and will not be updated again
    KeyMissmatch,  // Key has not been used
    Cancel,        // Cancel current operation and go to previous state
}

#[derive(Debug)]
pub struct ParamSelector {
    pub state: SelectorState,
    pub func_selection: ItemSelection,
    pub param_selection: ItemSelection,
    pub value: ParameterValue,

    // Used for modulation source/ target: When reaching this state, the value
    // is complete. For normal parameters, this will be Value, for modulation
    // parameters it is either FunctionIndex or Parameter.
    pub target_state: SelectorState,

    pub temp_string: String,
    pub sub_selector: Option<Rc<RefCell<ParamSelector>>>,
}

impl ParamSelector {
    /* Received a keyboard event from the terminal.
     *
     * Return true if a new value has been read completely, false otherwise.
     */
    pub fn handle_user_input(mut s: &mut ParamSelector,
                         c: termion::event::Key,
                         sound: &mut SoundData) -> bool {
        let mut key_consumed = false;
        let mut value_change_finished = false;

        while !key_consumed {
            info!("handle_user_input {:?} in state {:?}", c, s.state);
            key_consumed = true;
            let new_state = match s.state {

                // Select the function group to edit (Oscillator, Envelope, ...)
                SelectorState::Function => {
                    match ParamSelector::select_item(&mut s.func_selection, c) {
                        RetCode::KeyConsumed | RetCode::ValueUpdated  => s.state,       // Selection updated
                        RetCode::KeyMissmatch | RetCode::Cancel       => s.state,       // Ignore key that doesn't match a selection
                        RetCode::ValueComplete                        => next(s.state), // Function selected
                    }
                },

                // Select which item in the function group to edit (Oscillator 1, 2, 3, ...)
                SelectorState::FunctionIndex => {
                    match ParamSelector::get_value(s, c, sound) {
                        RetCode::KeyConsumed   => s.state,           // Key has been used, but value hasn't changed
                        RetCode::ValueUpdated  => s.state,           // Selection not complete yet
                        RetCode::ValueComplete => {                  // Parameter has been selected
                            // For modulation source or target, we might be finished here with
                            // getting the value. Compare current state to expected target state.
                            if s.state == s.target_state {
                                value_change_finished = true;
                                previous(s.state)
                            } else {
                                s.param_selection.item_list = s.func_selection.item_list[s.func_selection.item_index].next;
                                ParamSelector::select_param(&mut s, sound);
                                next(s.state)
                            }
                        },
                        RetCode::KeyMissmatch  => s.state,           // Ignore unmatched keys
                        RetCode::Cancel        => previous(s.state), // Abort function index selection
                    }
                },

                // Select the parameter of the function to edit (Waveshape, Frequency, ...)
                SelectorState::Param => {
                    match ParamSelector::select_item(&mut s.param_selection, c) {
                        RetCode::KeyConsumed   => s.state,           // Value has changed, but not complete yet
                        RetCode::ValueUpdated  => {                     // Pararmeter selection updated
                            ParamSelector::select_param(&mut s, sound);
                            s.state
                        },
                        RetCode::ValueComplete => {                     // Prepare to read the value
                            // For modulation source or target, we might be finished here with
                            // getting the value. Compare current state to expected target state.
                            if s.state == s.target_state {
                                value_change_finished = true;
                                previous(s.state)
                            } else {
                                ParamSelector::select_param(&mut s, sound);
                                next(s.state)
                            }
                        },
                        RetCode::KeyMissmatch  => s.state,           // Ignore invalid key
                        RetCode::Cancel        => previous(s.state), // Cancel parameter selection
                    }
                },

                // Select the parameter value
                SelectorState::Value => {
                    match ParamSelector::get_value(s, c, sound) {
                        RetCode::KeyConsumed   => s.state,
                        RetCode::ValueUpdated  => { // Value has changed to a valid value, update synth
                            value_change_finished = true;
                            s.state
                        },
                        RetCode::ValueComplete => {
                            value_change_finished = true;
                            previous(s.state) // Value has changed and will not be updated again
                        },
                        RetCode::KeyMissmatch  => {
                            // Key can't be used for value, so it probably is the short cut for a
                            // different parameter. Switch to parameter state and try again.
                            key_consumed = false;
                            previous(s.state)
                        },
                        RetCode::Cancel => {
                            // Stop updating the value, back to parameter selection
                            previous(s.state)
                        }
                    }
                }
            };
            ParamSelector::change_state(&mut s, new_state);
        }
        value_change_finished
    }

    /* Change the state of the input state machine. */
    fn change_state(selector: &mut ParamSelector, new_state: SelectorState) {
        if new_state != selector.state {
            if let SelectorState::Function = new_state {
                selector.func_selection.item_index = 0;
                selector.param_selection.item_index = 0;
            }
            info!("change_state {} -> {}", selector.state, new_state);
            selector.state = new_state;
        }
    }

    /* Select one of the items of functions or parameters.
     *
     * Called when a new user input is received and we're in the right state for function selection.
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
    fn get_value(s: &mut ParamSelector, c: termion::event::Key, sound: &mut SoundData) -> RetCode {
        let item: &mut ItemSelection;
        if s.state == SelectorState::FunctionIndex {
            item = &mut s.func_selection;
        } else {
            item = &mut s.param_selection;
        }
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
                                    RetCode::ValueComplete // Can't add another digit, accept value as final and move on
                                } else {
                                    RetCode::KeyConsumed   // Could add more digits, not finished yet
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
                    RetCode::ValueUpdated | RetCode::ValueComplete => ParamSelector::update_value(item, ParameterValue::Int(current), &mut s.temp_string),
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
                                s.temp_string.push(x);
                                let value: Result<Float, ParseFloatError> = s.temp_string.parse();
                                current = if let Ok(x) = value { x } else { current };
                                RetCode::KeyConsumed
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
                        let len = s.temp_string.len();
                        if len > 0 {
                            s.temp_string.pop();
                            if len >= 1 {
                                let value = s.temp_string.parse();
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
                    RetCode::ValueUpdated | RetCode::ValueComplete => ParamSelector::update_value(item, ParameterValue::Float(current), &mut s.temp_string),
                    _ => (),
                }
                result
            },
            ValueRange::ChoiceRange(choice_list) => {
                let mut current = if let ParameterValue::Choice(x) = item.value { x } else { panic!() };
                let result = match c {
                    Key::Up         => {current += 1; RetCode::ValueUpdated },
                    Key::Down       => if current > 0 { current -= 1; RetCode::ValueUpdated } else { RetCode::KeyConsumed },
                    Key::Left | Key::Backspace => RetCode::Cancel,
                    Key::Right      => RetCode::ValueComplete,
                    Key::Char('\n') => RetCode::ValueComplete,
                    _ => RetCode::KeyMissmatch,
                };
                match result {
                    RetCode::ValueUpdated | RetCode::ValueComplete => ParamSelector::update_value(item, ParameterValue::Choice(current), &mut s.temp_string),
                    _ => (),
                }
                result
            },
            ValueRange::FuncRange(_) | ValueRange::ParamRange(_) => {
                // Pass key to sub selector
                let result = match &mut s.sub_selector {
                    Some(sub) => {
                        info!("Calling sub-selector!");
                        let value_finished = ParamSelector::handle_user_input(&mut sub.borrow_mut(), c, sound);
                        info!("Sub-selector finished!");
                        if value_finished {
                            info!("Value finished!");
                            RetCode::ValueComplete
                        } else {
                            RetCode::KeyConsumed
                        }
                    },
                    None => panic!(),
                };
                if result == RetCode::ValueComplete {
                    let selector = if let Some(selector) = &s.sub_selector {selector} else {panic!()};
                    let selector = selector.borrow();
                    match s.param_selection.value {
                        ParameterValue::Function(ref mut v) => {
                            let s_f = &selector.func_selection;
                            v.function = s_f.item_list[s_f.item_index].item;
                            v.function_id = if let ParameterValue::Int(x) = s_f.value { x as usize } else { panic!() };
                        },
                        ParameterValue::Param(ref mut v) => {
                            let s_f = &selector.func_selection;
                            v.function = s_f.item_list[s_f.item_index].item;
                            v.function_id = if let ParameterValue::Int(x) = s_f.value { x as usize } else { panic!() };
                            let s_p = &selector.param_selection;
                            v.parameter = s_p.item_list[s_p.item_index].item;
                        },
                        _ => panic!(),
                    }
                    info!("Selector value = {:?}", s.param_selection.value);
                }
                result
            },
            _ => panic!(),
        }
    }

    /* Select the parameter chosen by input. */
    fn select_param(selector: &mut ParamSelector, sound: &SoundData) {
        info!("select_param {:?}", selector.param_selection.item_list[selector.param_selection.item_index].item);
        //let param = item.item_list[item.item_index].item;

        let function = &selector.func_selection.item_list[selector.func_selection.item_index];
        let function_id = if let ParameterValue::Int(x) = &selector.func_selection.value { *x as usize } else { panic!() };
        let parameter = &selector.param_selection.item_list[selector.param_selection.item_index];
        let param_val = &selector.param_selection.value;
        let param = SynthParam::new(function.item, function_id, parameter.item, *param_val);

        // The value in the selected parameter needs to point to the right type.
        // Initialize it with the minimum.
        let value = sound.get_value(&param);
        info!("Sound has value {:?}", value);
        selector.param_selection.value = match value {
            ParameterValue::Int(_) => value,
            ParameterValue::Float(_) => value,
            ParameterValue::Choice(_) => value,
            ParameterValue::Function(_) => {
                let sub_sel = if let Some(ref selector) = selector.sub_selector {selector} else {panic!()};
                sub_sel.borrow_mut().target_state = SelectorState::FunctionIndex;
                value
            },
            ParameterValue::Param(_) => {
                let sub_sel = if let Some(ref selector) = selector.sub_selector {selector} else {panic!()};
                sub_sel.borrow_mut().target_state = SelectorState::Param;
                value
            },
            ParameterValue::NoValue => panic!(),
        };
    }

    /* Store a new value in the selected parameter. */
    fn update_value(selection: &mut ItemSelection, val: ParameterValue, temp_string: &mut String) {
        info!("update_value item: {:?}, value: {:?}", selection.item_list[selection.item_index].item, val);
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
            ValueRange::FuncRange(selection_list) | ValueRange::ParamRange(selection_list) => {
                // These never get called. SubSelector always operates on Int(x)
                panic!();
            }
            ValueRange::NoRange => {}
        };
    }

    /* Evaluate the MIDI control change message (ModWheel) */
    pub fn handle_control_change(s: &mut ParamSelector, val: i64) -> bool {
        match s.state {
            SelectorState::Function => ParamSelector::change_state(s, SelectorState::FunctionIndex),
            SelectorState::FunctionIndex => (),
            SelectorState::Param => ParamSelector::change_state(s, SelectorState::Value),
            SelectorState::Value => (),
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
        //s.send_event();
        true
    }
}
