use super::Float;

use std::fmt::{self, Debug, Display};

use serde::{Serialize, Deserialize};
use log::{info, trace, warn};

#[derive(PartialEq, Clone, Copy, Debug, Eq, Hash, Serialize, Deserialize)]
pub enum Parameter {
    // Function
    Oscillator,
    Filter,
    Amp,
    Lfo,
    GlobalLfo,
    Envelope,
    Modulation,
    Delay,
    System,

    // Oscillator, Lfo
    Waveform,
    Level,
    WaveIndex,
    Frequency,
    Finetune,
    Sync,
    KeyFollow,
    Voices,
    Spread,

    // Filter
    Type,
    Cutoff,
    Q,
    Resonance,
    Gain,
    // Filter types
    RLPF,
    ResonZ,

    // Amp
    Volume,

    // Lfo

    // Envelope
    Attack,
    Decay,
    Sustain,
    Release,
    Factor,

    // Mod
    Source,
    Target,
    Amount,
    Active,

    // Waveforms
    Sine,
    Triangle,
    Saw,
    Square,
    Noise,
    SampleHold,

    // Delay
    Time,
    Feedback,
    Tone,

    // MIDI parameters
    KeyValue,
    KeyAttack,
    KeyAftertouch,
    Aftertouch,

    // System parameters
    Idle,
    Busy,
}

impl Default for Parameter {
    fn default() -> Self { Parameter::Oscillator }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct FunctionId {
    pub function: Parameter,
    pub function_id: usize,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct ParamId {
    pub function: Parameter,
    pub function_id: usize,
    pub parameter: Parameter
}

impl ParamId {
    pub fn new(function: Parameter, function_id: usize, parameter: Parameter) -> Self {
        ParamId{function, function_id, parameter}
    }

    pub fn new_from(from: &SynthParam) -> Self {
        ParamId{function: from.function, 
                function_id: from.function_id,
                parameter: from.parameter}
    }

    pub fn set(&mut self,
               func: Parameter,
               func_id: usize,
               param: Parameter) {
        self.function = func;
        self.function_id = func_id;
        self.parameter = param;
    }
}

#[derive(Clone, Copy, Debug)]
pub enum ParameterValue {
    Int(i64),
    Float(Float),
    Choice(usize),
    Function(FunctionId),
    Param(ParamId),
    NoValue
}

impl ParameterValue {
    pub fn as_float(&self) -> Float {
        match self {
            ParameterValue::Int(x) => *x as Float,
            ParameterValue::Float(x) => *x,
            ParameterValue::Choice(x) => *x as Float,
            _ => panic!("Cannot convert parameter value to float")
        }
    }

    pub fn set_from_float(&mut self, value: Float) {
        match self {
            ParameterValue::Int(x) => {*self = ParameterValue::Int(value as i64)},
            ParameterValue::Float(x) => {*self = ParameterValue::Float(value)},
            ParameterValue::Choice(x) => {*self = ParameterValue::Choice(value as usize)},
            _ => panic!("Cannot convert parameter value to float")
        }
    }
}

impl fmt::Display for Parameter {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

#[derive(Debug, Copy, Clone)]
pub struct SynthParam {
    pub function: Parameter,
    pub function_id: usize,
    pub parameter: Parameter,
    pub value: ParameterValue
}

impl SynthParam {
    pub fn new(function: Parameter,
               function_id: usize,
               parameter: Parameter,
               value: ParameterValue) -> Self {
        SynthParam{function, function_id, parameter, value}
    }

    pub fn new_from(from: &ParamId) -> Self {
        SynthParam{function: from.function, 
                   function_id: from.function_id,
                   parameter: from.parameter,
                   value: ParameterValue::NoValue}
    }

    pub fn equals(&self, other: &ParamId) -> bool {
        self.function == other.function &&
        self.function_id == other.function_id &&
        self.parameter == other.parameter
    }
}

/** Enum for ranges of valid values */
#[derive(Clone, Copy, Debug)]
pub enum ValueRange {
    IntRange(i64, i64),               // Range (min, max) of integer values
    FloatRange(Float, Float, Float),  // Range (min, max, step) of float values
    ChoiceRange(&'static [MenuItem]), // A list of items to choose from
    FuncRange(&'static [MenuItem]),   // A list of (function-id) function entries
    ParamRange(&'static [MenuItem]),  // A list of (function-id-param) parameter entries
    NoRange
}

impl ValueRange {

    /** Translates an integer value into a parameter value of the value range.
     *
     * This is currently only used for controller values in the range 0 - 127.
     */
    pub fn translate_value(&self, val: u64) -> ParameterValue {
        match self {
            ValueRange::IntRange(min, max) => {
                let inc: Float = (max - min) as Float / 127.0;
                let value = min + (val as Float * inc) as i64;
                ParameterValue::Int(value)
            }
            ValueRange::FloatRange(min, max, _) => {
                let inc: Float = (max - min) / 127.0;
                let value = min + val as Float * inc;
                ParameterValue::Float(value)
            }
            ValueRange::ChoiceRange(choice_list) => {
                let inc: Float = choice_list.len() as Float / 127.0;
                let value = (val as Float * inc) as i64;
                ParameterValue::Choice(value as usize)
            }
            _ => ParameterValue::NoValue
        }
    }

    /** Adds or subtracts two integers if the result is within the given range. */
    pub fn add_value(&self, val: ParameterValue, addsub: i64) -> ParameterValue {
        match self {
            ValueRange::IntRange(min, max) => {
                let mut value = if let ParameterValue::Int(x) = val {
                    x
                } else {
                    panic!()
                };
                let result = value + addsub;
                if result >= *min && result <= *max {
                    value = result;
                }
                ParameterValue::Int(value)
            }
            ValueRange::FloatRange(min, max, step) => {
                let mut value = if let ParameterValue::Float(x) = val {
                    x
                } else {
                    panic!()
                };
                let result = value + (addsub as Float * step);
                if result >= *min && result <= *max {
                    value = result;
                }
                ParameterValue::Float(value)
            }
            ValueRange::ChoiceRange(choice_list) => {
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
            _ => ParameterValue::NoValue
        }
    }

    pub fn get_min_max(&self) -> (Float, Float) {
        match self {
            ValueRange::IntRange(min, max) => (*min as Float, *max as Float),
            ValueRange::FloatRange(min, max, _) => (*min, *max),
            ValueRange::ChoiceRange(itemlist) => (0.0, itemlist.len() as Float),
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

#[derive(Debug)]
pub enum ModFunction {
    Source,
    Target,
    NoMod,
}

/* Item for a list of selectable functions */
#[derive(Debug)]
pub struct MenuItem {
    pub item: Parameter,
    pub key: char,
    pub val_range: ValueRange,
    pub next: &'static [MenuItem]
}

impl MenuItem {
    pub fn get_val_range(function: Parameter, parameter: Parameter) -> &'static ValueRange {
        let func_item = MenuItem::get_menu_item(function, &FUNCTIONS);
        let param_item = MenuItem::get_menu_item(parameter, &func_item.next);
        &param_item.val_range
    }

    fn get_menu_item(item: Parameter, item_list: &'static [MenuItem]) -> &'static MenuItem {
        info!("Looking up MenuItem for item {}", item);
        for (i, s) in item_list.iter().enumerate() {
            if s.item == item {
                return &s;
            }
        }
        panic!();
    }
}

/* Top-level menu */
pub static FUNCTIONS: [MenuItem; 7] = [
    MenuItem{item: Parameter::Oscillator, key: 'o', val_range: ValueRange::IntRange(1, 3),  next: &OSC_PARAMS},
    MenuItem{item: Parameter::Envelope,   key: 'e', val_range: ValueRange::IntRange(1, 2),  next: &ENV_PARAMS},
    MenuItem{item: Parameter::Lfo,        key: 'l', val_range: ValueRange::IntRange(1, 2),  next: &LFO_PARAMS},
    MenuItem{item: Parameter::GlobalLfo,  key: 'g', val_range: ValueRange::IntRange(1, 2),  next: &LFO_PARAMS},
    MenuItem{item: Parameter::Filter,     key: 'f', val_range: ValueRange::IntRange(1, 2),  next: &FILTER_PARAMS},
    MenuItem{item: Parameter::Delay,      key: 'd', val_range: ValueRange::IntRange(1, 1),  next: &DELAY_PARAMS},
    MenuItem{item: Parameter::Modulation, key: 'm', val_range: ValueRange::IntRange(1, 16), next: &MOD_PARAMS},
];

pub static OSC_PARAMS: [MenuItem; 8] = [
    MenuItem{item: Parameter::Level,     key: 'l', val_range: ValueRange::FloatRange(0.0, 100.0, 1.0),  next: &[]},
    MenuItem{item: Parameter::WaveIndex, key: 'w', val_range: ValueRange::FloatRange(0.0, 1.0, 0.01),   next: &[]},
    MenuItem{item: Parameter::Frequency, key: 'f', val_range: ValueRange::IntRange(-24, 24),            next: &[]},
    MenuItem{item: Parameter::Finetune,  key: 't', val_range: ValueRange::FloatRange(0.0, 1200.0, 1.0), next: &[]},
    MenuItem{item: Parameter::Sync,      key: 's', val_range: ValueRange::IntRange(0, 1),               next: &[]},
    MenuItem{item: Parameter::KeyFollow, key: 'k', val_range: ValueRange::IntRange(0, 1),               next: &[]},
    MenuItem{item: Parameter::Voices,    key: 'v', val_range: ValueRange::IntRange(1, 7),               next: &[]},
    MenuItem{item: Parameter::Spread,    key: 'e', val_range: ValueRange::FloatRange(0.0, 2.0, 0.1),    next: &[]},
];

pub static LFO_PARAMS: [MenuItem; 2] = [
    MenuItem{item: Parameter::Waveform,  key: 'w', val_range: ValueRange::ChoiceRange(&LFO_WAVEFORM),    next: &[]},
    MenuItem{item: Parameter::Frequency, key: 'f', val_range: ValueRange::FloatRange(0.0, 22000.0, 1.0), next: &[]},
    //MenuItem{item: Parameter::Phase,     key: 'p', val_range: ValueRange::FloatRange(0.0, 100.0),        next: &[]},
];

pub static FILTER_PARAMS: [MenuItem; 4] = [
    MenuItem{item: Parameter::Type,      key: 't', val_range: ValueRange::ChoiceRange(&FILTER_TYPE),    next: &[]},
    MenuItem{item: Parameter::Cutoff,    key: 'c', val_range: ValueRange::FloatRange(1.0, 5000.0, 1.0), next: &[]},
    MenuItem{item: Parameter::Resonance, key: 'r', val_range: ValueRange::FloatRange(1.0, 100.0, 1.0),  next: &[]},
    MenuItem{item: Parameter::Gain,      key: 'g', val_range: ValueRange::FloatRange(0.0, 1.0, 0.01),   next: &[]},
];

pub static FILTER_TYPE: [MenuItem; 2] = [
    MenuItem{item: Parameter::RLPF,      key: 'l', val_range: ValueRange::NoRange, next: &[]},
    MenuItem{item: Parameter::ResonZ,    key: 'r', val_range: ValueRange::NoRange, next: &[]},
];

pub static ENV_PARAMS: [MenuItem; 5] = [
    MenuItem{item: Parameter::Attack,  key: 'a', val_range: ValueRange::FloatRange(0.0, 4000.0, 1.0), next: &[]}, // Value = Duration in ms
    MenuItem{item: Parameter::Decay,   key: 'd', val_range: ValueRange::FloatRange(0.0, 4000.0, 1.0), next: &[]},
    MenuItem{item: Parameter::Sustain, key: 's', val_range: ValueRange::FloatRange(0.0, 1.0, 0.001),  next: &[]},
    MenuItem{item: Parameter::Release, key: 'r', val_range: ValueRange::FloatRange(0.0, 8000.0, 1.0), next: &[]},
    MenuItem{item: Parameter::Factor,  key: 'f', val_range: ValueRange::IntRange(1, 5),               next: &[]},
];

pub static OSC_WAVEFORM: [MenuItem; 5] = [
    MenuItem{item: Parameter::Sine,      key: 's', val_range: ValueRange::NoRange, next: &[]},
    MenuItem{item: Parameter::Triangle,  key: 't', val_range: ValueRange::NoRange, next: &[]},
    MenuItem{item: Parameter::Saw,       key: 'w', val_range: ValueRange::NoRange, next: &[]},
    MenuItem{item: Parameter::Square,    key: 'q', val_range: ValueRange::NoRange, next: &[]},
    MenuItem{item: Parameter::Noise ,    key: 'n', val_range: ValueRange::NoRange, next: &[]},
];

pub static LFO_WAVEFORM: [MenuItem; 6] = [
    MenuItem{item: Parameter::Sine,      key: 's', val_range: ValueRange::NoRange, next: &[]},
    MenuItem{item: Parameter::Triangle,  key: 't', val_range: ValueRange::NoRange, next: &[]},
    MenuItem{item: Parameter::Saw,       key: 'w', val_range: ValueRange::NoRange, next: &[]},
    MenuItem{item: Parameter::Square,    key: 'q', val_range: ValueRange::NoRange, next: &[]},
    MenuItem{item: Parameter::Noise ,    key: 'n', val_range: ValueRange::NoRange, next: &[]},
    MenuItem{item: Parameter::SampleHold,key: 'h', val_range: ValueRange::NoRange, next: &[]},
];

pub static DELAY_PARAMS: [MenuItem; 4] = [
    MenuItem{item: Parameter::Time,      key: 't', val_range: ValueRange::FloatRange(0.01, 1.0, 0.01),    next: &[]},
    MenuItem{item: Parameter::Level,     key: 'l', val_range: ValueRange::FloatRange(0.0, 1.0, 0.01),     next: &[]},
    MenuItem{item: Parameter::Feedback,  key: 'f', val_range: ValueRange::FloatRange(0.0, 1.0, 0.01),     next: &[]},
    MenuItem{item: Parameter::Tone,      key: 'o', val_range: ValueRange::FloatRange(100.0, 5000.0, 1.0), next: &[]},
];

pub static MOD_PARAMS: [MenuItem; 4] = [
    MenuItem{item: Parameter::Source,    key: 's', val_range: ValueRange::FuncRange(&MOD_SOURCES),    next: &MOD_SOURCES},
    MenuItem{item: Parameter::Target,    key: 't', val_range: ValueRange::ParamRange(&MOD_TARGETS),   next: &MOD_TARGETS},
    MenuItem{item: Parameter::Amount,    key: 'a', val_range: ValueRange::FloatRange(0.0, 1.0, 0.01), next: &[]},
    MenuItem{item: Parameter::Active,    key: 'v', val_range: ValueRange::IntRange(0, 1),             next: &[]},
];

pub static MOD_SOURCES: [MenuItem; 4] = [
    MenuItem{item: Parameter::Oscillator, key: 'o', val_range: ValueRange::IntRange(1, 3), next: &OSC_PARAMS},
    MenuItem{item: Parameter::Envelope,   key: 'e', val_range: ValueRange::IntRange(1, 2), next: &ENV_PARAMS},
    MenuItem{item: Parameter::Lfo,        key: 'l', val_range: ValueRange::IntRange(1, 2), next: &LFO_PARAMS},
    MenuItem{item: Parameter::GlobalLfo,  key: 'g', val_range: ValueRange::IntRange(1, 2), next: &LFO_PARAMS},
];

pub static MOD_TARGETS: [MenuItem; 6] = [
    MenuItem{item: Parameter::Oscillator, key: 'o', val_range: ValueRange::IntRange(1, 3), next: &OSC_PARAMS},
    MenuItem{item: Parameter::Envelope,   key: 'e', val_range: ValueRange::IntRange(1, 2), next: &ENV_PARAMS},
    MenuItem{item: Parameter::Lfo,        key: 'l', val_range: ValueRange::IntRange(1, 2), next: &LFO_PARAMS},
    MenuItem{item: Parameter::GlobalLfo,  key: 'g', val_range: ValueRange::IntRange(1, 2), next: &LFO_PARAMS},
    MenuItem{item: Parameter::Filter,     key: 'f', val_range: ValueRange::IntRange(1, 2), next: &FILTER_PARAMS},
    MenuItem{item: Parameter::Delay,      key: 'd', val_range: ValueRange::IntRange(1, 1), next: &DELAY_PARAMS},
];

