use super::Float;
use super::parameter::{Parameter, ParameterValue, MenuItem};

/** Enum for ranges of valid values */
#[derive(Clone, Copy, Debug)]
pub enum ValueRange {
    Int(i64, i64),               // Range (min, max) of integer values
    Float(Float, Float, Float),  // Range (min, max, step) of float values
    Choice(&'static [MenuItem]), // A list of items to choose from
    Func(&'static [MenuItem]),   // A list of (function-id) function entries
    Param(&'static [MenuItem]),  // A list of (function-id-param) parameter entries
    Dynamic(Parameter),          // List is dynamically generated according to the ID
    NoRange
}

impl ValueRange {

    /** Translates an integer value into a parameter value of the value range.
     *
     * This is currently only used for controller values in the range 0 - 127.
     */
    pub fn translate_value(&self, val: u64) -> ParameterValue {
        match self {
            ValueRange::Int(min, max) => {
                let inc: Float = (max - min) as Float / 127.0;
                let value = min + (val as Float * inc) as i64;
                ParameterValue::Int(value)
            }
            ValueRange::Float(min, max, _) => {
                let inc: Float = (max - min) / 127.0;
                let value = min + val as Float * inc;
                ParameterValue::Float(value)
            }
            ValueRange::Choice(choice_list) => {
                let inc: Float = choice_list.len() as Float / 127.0;
                let value = (val as Float * inc) as i64;
                ParameterValue::Choice(value as usize)
            }
            ValueRange::Dynamic(param) => {
                ParameterValue::Dynamic(*param, val as usize)
            }
            _ => ParameterValue::NoValue
        }
    }

    /** Adds or subtracts two integers if the result is within the given range. */
    pub fn add_value(&self, val: ParameterValue, addsub: i64) -> ParameterValue {
        match self {
            ValueRange::Int(min, max) => {
                let mut value = if let ParameterValue::Int(x) = val {
                    x
                } else {
                    panic!()
                };
                let result = value + addsub;
                value = if result >= *max {
                    *max
                } else if result <= *min {
                    *min
                } else {
                    result
                };
                ParameterValue::Int(value)
            }
            ValueRange::Float(min, max, step) => {
                let mut value = if let ParameterValue::Float(x) = val {
                    x
                } else {
                    panic!()
                };
                let result = value + (addsub as Float * step);
                value = if result >= *max {
                    *max
                } else if result <= *min {
                    *min
                } else {
                    result
                };
                ParameterValue::Float(value)
            }
            ValueRange::Choice(choice_list) => {
                let mut value = if let ParameterValue::Choice(x) = val {
                    x
                } else {
                    panic!()
                };
                let result = value + addsub as usize;
                if result < choice_list.len() {
                    value = result;
                }
                ParameterValue::Choice(value)
            }
            ValueRange::Dynamic(param) => {
                let value = if let ParameterValue::Dynamic(_p, x) = val {
                    x
                } else {
                    panic!()
                };
                let result = if addsub > 0 || value > 0 {
                    value + addsub as usize
                } else {
                    value
                };
                ParameterValue::Dynamic(*param, result)
            }
            _ => ParameterValue::NoValue
        }
    }

    pub fn get_min_max(&self) -> (Float, Float) {
        match self {
            ValueRange::Int(min, max) => (*min as Float, *max as Float),
            ValueRange::Float(min, max, _) => (*min, *max),
            ValueRange::Choice(itemlist) => (0.0, itemlist.len() as Float),
            _ => panic!("Unexpected value range, cannot get min and max"),
        }
    }

    /** Adds two floats, keeps result within value range. */
    pub fn safe_add(&self, a: Float, b: Float) -> Float {
        let result = a + b;
        let (min, max) = self.get_min_max();
        if result < min {
            min
        } else if result > max {
            max
        } else {
            result
        }
    }
}

impl Default for ValueRange {
    fn default() -> Self { ValueRange::NoRange }
}

