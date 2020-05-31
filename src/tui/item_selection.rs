use super::{FunctionId, ParameterValue, ValueRange, MenuItem, RetCode};
use super::Float;

use log::info;
use termion::event::Key;

use std::num::ParseIntError;
use std::num::ParseFloatError;

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

    pub fn set_list_from(&mut self, from: &ItemSelection, item_index: usize) {
        self.item_list = from.item_list[from.item_index].next;
        self.item_index = item_index;
    }

    /** Select one of the items in the function list.
     *
     * Called when a new user input is received and we're in the right state
     * for function selection. Updates the item_index of the ItemSelection.
     */
    pub fn select_item(&mut self, c: termion::event::Key) -> RetCode {
        let result = match c {
            Key::Up => {
                if self.item_index < self.item_list.len() - 1 {
                    self.item_index += 1;
                }
                RetCode::ValueUpdated
            },
            Key::Down => {
                if self.item_index > 0 {
                    self.item_index -= 1;
                }
                RetCode::ValueUpdated
            },
            Key::Left | Key::Backspace => RetCode::Cancel,
            Key::Right => RetCode::ValueComplete,
            Key::Esc => RetCode::Reset,
            Key::Char('\n') => RetCode::ValueComplete,
            Key::Char(c) => {
                for (count, f) in self.item_list.iter().enumerate() {
                    if f.key == c {
                        self.item_index = count;
                        return RetCode::ValueComplete;
                    }
                }
                RetCode::KeyConsumed
            },
            _ => RetCode::KeyConsumed
        };
        info!("select_item {:?} (index {})", self.item_list[self.item_index].item, self.item_index);
        result
    }

    /** Get the value range for the currently selected parameter. */
    pub fn get_val_range(&self) -> &'static ValueRange {
        &self.item_list[self.item_index].val_range
    }

    /** Evaluate the MIDI control change message (ModWheel) */
    pub fn set_control_value(&mut self, val: u64) {
        let value = self.get_val_range().translate_value(val);
        self.update_value(value);
    }

    /** Handle an input event for changing the selected parameter.
     *
     * Supports entering the value by
     * - Direct ascii input of the number
     * - Adjusting current value with Up/ Down, +/ - keys
     *
     * \todo Passing the wavetable list length as parameter is an ugly hack,
     *       need to find something better.
     */
    pub fn handle_input(&mut self, c: termion::event::Key, dyn_list_len: usize) -> RetCode {
        info!("handle_input {:?}", self.item_list[self.item_index].item);

        match c {
            // Common keys
            Key::Esc   => RetCode::Reset,

            // All others
            _ => {
                match self.item_list[self.item_index].val_range {
                    ValueRange::Int(min, max) => self.handle_input_for_int(min, max, c),
                    ValueRange::Float(_min, _max, _step) => self.handle_input_for_float(c),
                    ValueRange::Choice(_choice_list) => self.handle_input_for_choice(c),
                    ValueRange::Dynamic(_param) => self.handle_input_for_dynamic(dyn_list_len, c), // TODO: Use a closure as parameter instead
                    _ => panic!(),
                }
            }
        }
    }

    /** Received a key for changing an int value. */
    fn handle_input_for_int(&mut self,
                            min: i64,
                            max: i64,
                            c: termion::event::Key) -> RetCode {
        let mut current = if let ParameterValue::Int(x) = self.value { x } else { panic!() };
        let result = match c {
            Key::Char(x) => {
                match x {
                    '0' | '1' | '2' | '3' | '4' | '5' | '6' | '7' | '8' | '9' | '.' => {
                        let y = x as i64 - '0' as i64;
                        if self.temp_string.len() > 0 {
                            // Something already in temp string, append to end if possible.
                            let current_temp: Result<i64, ParseIntError> = self.temp_string.parse();
                            let current_temp = if let Ok(x) = current_temp {
                                x
                            } else {
                                self.temp_string.clear();
                                0
                            };
                            let val_digit_added = current_temp * 10 + y;
                            if val_digit_added > max {
                                // Value would be too big, ignore key
                                RetCode::KeyConsumed
                            } else {
                                self.temp_string.push(x);
                                let value: Result<i64, ParseIntError> = self.temp_string.parse();
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
                                self.temp_string.push(x);
                                let value: Result<i64, ParseIntError> = self.temp_string.parse();
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
            RetCode::ValueUpdated | RetCode::ValueComplete => self.update_value(ParameterValue::Int(current)),
            _ => (),
        }
        result
    }

    /** Received a key for changing a float value. */
    fn handle_input_for_float(&mut self,
                              c: termion::event::Key) -> RetCode {
        let mut current = if let ParameterValue::Float(x) = self.value { x } else { panic!() };
        let result = match c {
            Key::Char(x) => {
                match x {
                    '0' | '1' | '2' | '3' | '4' | '5' | '6' | '7' | '8' | '9' | '.' => {
                        self.temp_string.push(x);
                        let value: Result<Float, ParseFloatError> = self.temp_string.parse();
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
                let len = self.temp_string.len();
                if len > 0 {
                    self.temp_string.pop();
                    if self.temp_string.len() >= 1 {
                        let value = self.temp_string.parse();
                        current = if let Ok(x) = value { x } else { current };
                    } else {
                        self.temp_string.push('0');
                        current = 0.0;
                    }
                } else {
                    self.temp_string.push('0');
                    current = 0.0;
                }
                RetCode::ValueUpdated
            },
            _ => RetCode::KeyMissmatch,
        };
        match result {
            RetCode::ValueUpdated | RetCode::ValueComplete => self.update_value(ParameterValue::Float(current)),
            _ => (),
        }
        result
    }

    /** Received a key for selecting an item from a static list. */
    fn handle_input_for_choice(&mut self,
                               c: termion::event::Key) -> RetCode {
        let mut current = if let ParameterValue::Choice(x) = self.value { x } else { panic!() };
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
            RetCode::ValueUpdated | RetCode::ValueComplete => self.update_value(ParameterValue::Choice(current)),
            _ => (),
        }
        result
    }

    /** Received a key for selecting an item from a dynamic list. */
    fn handle_input_for_dynamic(&mut self,
                                max: usize,
                                c: termion::event::Key) -> RetCode {
        let (param, mut current) = if let ParameterValue::Dynamic(p, x) = self.value { (p, x) } else { panic!() };
        let result = match c {
            Key::Char('+') |
            Key::Up         => if current < max { current += 1; RetCode::ValueUpdated } else { RetCode::KeyConsumed },
            Key::Char('-') |
            Key::Down       => if current > 0 { current -= 1; RetCode::ValueUpdated } else { RetCode::KeyConsumed },
            Key::Left       => RetCode::Cancel,
            Key::Right      => RetCode::ValueComplete,
            Key::Backspace  => RetCode::Cancel,
            Key::Char('\n') => RetCode::ValueComplete,
            _ => RetCode::KeyMissmatch,
        };
        match result {
            RetCode::ValueUpdated | RetCode::ValueComplete => self.update_value(ParameterValue::Dynamic(param, current)),
            _ => (),
        }
        result
    }

    /** Store a new value in the selected parameter. */
    pub fn update_value(&mut self, val: ParameterValue) {
        info!("update_value item: {:?}, value: {:?}", self.item_list[self.item_index].item, val);
        match self.get_val_range() {
            ValueRange::Int(min, max) => {
                let mut value = if let ParameterValue::Int(x) = val { x } else { panic!(); };
                if value > *max {
                    value = *max;
                }
                if value < *min {
                    value = *min;
                }
                self.value = ParameterValue::Int(value);
            }
            ValueRange::Float(min, max, _) => {
                let mut value = if let ParameterValue::Float(x) = val { x } else { panic!(); };
                let has_period =  self.temp_string.contains(".");
                if value > *max {
                    value = *max;
                }
                if value < *min {
                    value = *min;
                }
                self.temp_string.clear();
                self.temp_string.push_str(value.to_string().as_str());
                if !self.temp_string.contains(".") && has_period {
                    self.temp_string.push('.');
                }
                self.value = ParameterValue::Float(value);
            }
            ValueRange::Choice(selection_list) => {
                let mut value = if let ParameterValue::Choice(x) = val { x as usize } else { panic!("{:?}", val); };
                if value >= selection_list.len() {
                    value = selection_list.len() - 1;
                }
                self.value = ParameterValue::Choice(value);
            }
            ValueRange::Dynamic(_id) => {
                self.value = val;
            }
            ValueRange::Func(_selection_list) => {
                //panic!();
            }
            ValueRange::Param(_selection_list) => {
                //panic!();
            }
            ValueRange::NoRange => {}
        };
    }
}

