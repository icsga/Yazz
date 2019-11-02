use super::parameter::{MenuItem, ParameterValue, FUNCTIONS};

use std::rc::Rc;

#[derive(Copy, Clone, PartialEq, Debug)]
enum SelectionState {
    Function,
    FunctionIndex,
    Param,
    Value,
}

impl fmt::Display for SelectionState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

fn next(current: SelectionState) -> SelectionState {
    use SelectionState::*;
    match current {
        Function => FunctionIndex,
        FunctionIndex => Param,
        Param => Value,
        Value => Param,
    }
}

fn previous(current: SelectionState) -> SelectionState {
    use SelectionState::*;
    match current {
        Function => Function,
        FunctionIndex => Function,
        Param => FunctionIndex,
        Value => Param,
    }
}

enum ValueHolder {
    Value(ParameterValue),
    SubSelection(Rc<ParamSelection>),
}

pub struct ParamSelection {
    state: SelectionState,
    function_list: &'static [MenuItem],
    function_index: usize,              // Index into function_list
    function_id: usize,                 // ID of chosen function (e.g. OSC 1)
    param_list: &'static [MenuItem],
    param_index: usize,                 // Inde into param_list
    pub value: ValueHolder,             // Wrapper around the actual value or substate
    temp_string: String,
}

impl ParamSelection {
    pub fn new() -> ParamSelection {
        ParamSelection{
            state: SelectionState::Function,
            function_list: &FUNCTIONS,
            function_index: 0,
            function_id: 0,
            param_list: &[], // Will be set when selecting a function
            param_index: 0,
            param_id: 0,
            value: ValueHolder::Value(ParameterValue::Int(0)),
            temp_string: String::new(),
        }
    }

    pub fn handle_control_change(&mut self, val: i64) {
        match self.state {
            SelectionState::Function => self.change_state(SelectionState::FunctionIndex),
            SelectionState::FunctionIndex => (),
            SelectionState::Param => self.change_state(SelectionState::Value),
            SelectionState::Value => (),
            SelectionState::ValueSubFunction => self.change_state(SelectionState::ValueSubFunctionIndex),
            SelectionState::ValueSubFunctionIndex => (),
            SelectionState::ValueSubParam => (),
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

    pub fn handle_synth_param(&mut self, m: SynthParam) {
        info!("handle_synth_param {} = {:?}", self.selected_param.item_list[self.selected_param.item_index].item, m);
        let item = &mut self.selected_param;
        Tui::update_value(item, m.value, &mut self.temp_string);
    }

    /* Received a keyboard event from the terminal. */
    fn handle_user_input(&mut self, c: termion::event::Key) {
        let mut key_consumed = false;

        while !key_consumed {
            info!("handle_user_input {:?}", c);
            key_consumed = true;
            let new_state = match self.state {

                // Select the function group to edit (Oscillator, Envelope, ...)
                SelectionState::Function => {
                    match Tui::select_item(&mut self.selected_function, c) {
                        ReturnCode::KeyConsumed | ReturnCode::ValueUpdated  => self.state,       // Selection updated
                        ReturnCode::KeyMissmatch | ReturnCode::Cancel       => self.state,       // Ignore key that doesn't match a selection
                        ReturnCode::ValueComplete                           => next(self.state), // Function selected
                    }
                },

                // Select which item in the function group to edit (Oscillator 1, 2, 3, ...)
                SelectionState::FunctionIndex => {
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
                SelectionState::Param => {
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
                SelectionState::Value => {
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
            };
            self.change_state(new_state);
        }
    }

    /* Change the state of the input state machine. */
    fn change_state(&mut self, mut new_state: SelectionState) {
        if new_state != self.state {
            match new_state {
                SelectionState::Function => {
                    // We are probably selecting a different function than
                    // before, so we should start the parameter list at the
                    // beginning to avoid out-of-bound errors.
                    self.selected_param.item_index = 0;
                }
                SelectionState::FunctionIndex => {}
                SelectionState::Param => {}
                SelectionState::Value => {
                }
            }
            info!("change_state {} -> {}", self.state, new_state);
            self.state = new_state;
        }
    }

}

