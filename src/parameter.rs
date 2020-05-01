use super::Float;
use super::synth::*;
use super::voice::*;

use std::fmt::{self, Debug, Display};

use serde::{Serialize, Deserialize};
use log::{info, trace, warn};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
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
    Wavetable,
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
    Velocity,
    KeyAftertouch,
    Aftertouch,
    PitchWheel,
    ModWheel,

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

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq, Serialize, Deserialize)]
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
    Dynamic(Parameter, usize),
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

#[derive(Clone, Copy, Debug)]
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
                if result >= *min && result <= *max {
                    value = result;
                }
                ParameterValue::Int(value)
            }
            ValueRange::Float(min, max, step) => {
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
    MenuItem{item: Parameter::Oscillator, key: 'o', val_range: ValueRange::Int(1, NUM_OSCILLATORS as i64),  next: &OSC_PARAMS},
    MenuItem{item: Parameter::Envelope,   key: 'e', val_range: ValueRange::Int(1, NUM_ENVELOPES as i64),  next: &ENV_PARAMS},
    MenuItem{item: Parameter::Lfo,        key: 'l', val_range: ValueRange::Int(1, NUM_LFOS as i64),  next: &LFO_PARAMS},
    MenuItem{item: Parameter::GlobalLfo,  key: 'g', val_range: ValueRange::Int(1, NUM_GLOBAL_LFOS as i64),  next: &LFO_PARAMS},
    MenuItem{item: Parameter::Filter,     key: 'f', val_range: ValueRange::Int(1, NUM_FILTERS as i64),  next: &FILTER_PARAMS},
    MenuItem{item: Parameter::Delay,      key: 'd', val_range: ValueRange::Int(1, 1),  next: &DELAY_PARAMS},
    MenuItem{item: Parameter::Modulation, key: 'm', val_range: ValueRange::Int(1, NUM_MODULATORS as i64), next: &MOD_PARAMS},
];

pub static OSC_PARAMS: [MenuItem; 9] = [
    MenuItem{item: Parameter::Level,     key: 'l', val_range: ValueRange::Float(0.0, 100.0, 1.0),       next: &[]},
    MenuItem{item: Parameter::Wavetable, key: 'w', val_range: ValueRange::Dynamic(Parameter::Wavetable),next: &[]},
    MenuItem{item: Parameter::WaveIndex, key: 'i', val_range: ValueRange::Float(0.0, 1.0, 0.01),        next: &[]},
    MenuItem{item: Parameter::Frequency, key: 'f', val_range: ValueRange::Int(-24, 24),                 next: &[]},
    MenuItem{item: Parameter::Finetune,  key: 't', val_range: ValueRange::Float(-100.0, 100.0, 1.0),    next: &[]},
    MenuItem{item: Parameter::Sync,      key: 's', val_range: ValueRange::Int(0, 1),                    next: &[]},
    MenuItem{item: Parameter::KeyFollow, key: 'k', val_range: ValueRange::Int(0, 1),                    next: &[]},
    MenuItem{item: Parameter::Voices,    key: 'v', val_range: ValueRange::Int(1, 7),                    next: &[]},
    MenuItem{item: Parameter::Spread,    key: 'e', val_range: ValueRange::Float(0.0, 2.0, 0.1),         next: &[]},
];

pub static LFO_PARAMS: [MenuItem; 2] = [
    MenuItem{item: Parameter::Waveform,  key: 'w', val_range: ValueRange::Choice(&LFO_WAVEFORM),    next: &[]},
    MenuItem{item: Parameter::Frequency, key: 'f', val_range: ValueRange::Float(0.0, 22000.0, 1.0), next: &[]},
];

pub static FILTER_PARAMS: [MenuItem; 4] = [
    MenuItem{item: Parameter::Type,      key: 't', val_range: ValueRange::Choice(&FILTER_TYPE),    next: &[]},
    MenuItem{item: Parameter::Cutoff,    key: 'c', val_range: ValueRange::Float(1.0, 5000.0, 1.0), next: &[]},
    MenuItem{item: Parameter::Resonance, key: 'r', val_range: ValueRange::Float(1.0, 100.0, 1.0),  next: &[]},
    MenuItem{item: Parameter::Gain,      key: 'g', val_range: ValueRange::Float(0.0, 1.0, 0.01),   next: &[]},
];

pub static FILTER_TYPE: [MenuItem; 2] = [
    MenuItem{item: Parameter::RLPF,      key: 'l', val_range: ValueRange::NoRange, next: &[]},
    MenuItem{item: Parameter::ResonZ,    key: 'r', val_range: ValueRange::NoRange, next: &[]},
];

pub static ENV_PARAMS: [MenuItem; 5] = [
    MenuItem{item: Parameter::Attack,  key: 'a', val_range: ValueRange::Float(0.0, 4000.0, 1.0), next: &[]}, // Value = Duration in ms
    MenuItem{item: Parameter::Decay,   key: 'd', val_range: ValueRange::Float(0.0, 4000.0, 1.0), next: &[]},
    MenuItem{item: Parameter::Sustain, key: 's', val_range: ValueRange::Float(0.0, 1.0, 0.001),  next: &[]},
    MenuItem{item: Parameter::Release, key: 'r', val_range: ValueRange::Float(0.0, 8000.0, 1.0), next: &[]},
    MenuItem{item: Parameter::Factor,  key: 'f', val_range: ValueRange::Int(1, 5),               next: &[]},
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
    MenuItem{item: Parameter::SampleHold,key: 'h', val_range: ValueRange::NoRange, next: &[]},
    MenuItem{item: Parameter::Noise ,    key: 'n', val_range: ValueRange::NoRange, next: &[]},
];

pub static DELAY_PARAMS: [MenuItem; 4] = [
    MenuItem{item: Parameter::Time,      key: 't', val_range: ValueRange::Float(0.01, 1.0, 0.01),    next: &[]},
    MenuItem{item: Parameter::Level,     key: 'l', val_range: ValueRange::Float(0.0, 1.0, 0.01),     next: &[]},
    MenuItem{item: Parameter::Feedback,  key: 'f', val_range: ValueRange::Float(0.0, 1.0, 0.01),     next: &[]},
    MenuItem{item: Parameter::Tone,      key: 'o', val_range: ValueRange::Float(100.0, 5000.0, 1.0), next: &[]},
];

pub static MOD_PARAMS: [MenuItem; 4] = [
    MenuItem{item: Parameter::Source,    key: 's', val_range: ValueRange::Func(&MOD_SOURCES),    next: &MOD_SOURCES},
    MenuItem{item: Parameter::Target,    key: 't', val_range: ValueRange::Param(&MOD_TARGETS),   next: &MOD_TARGETS},
    MenuItem{item: Parameter::Amount,    key: 'a', val_range: ValueRange::Float(0.0, 1.0, 0.01), next: &[]},
    MenuItem{item: Parameter::Active,    key: 'v', val_range: ValueRange::Int(0, 1),             next: &[]},
];

pub static MOD_SOURCES: [MenuItem; 8] = [
    MenuItem{item: Parameter::Oscillator, key: 'o', val_range: ValueRange::Int(1, 3), next: &OSC_PARAMS},
    MenuItem{item: Parameter::Envelope,   key: 'e', val_range: ValueRange::Int(1, 2), next: &ENV_PARAMS},
    MenuItem{item: Parameter::Lfo,        key: 'l', val_range: ValueRange::Int(1, 2), next: &LFO_PARAMS},
    MenuItem{item: Parameter::Velocity,   key: 'v', val_range: ValueRange::Int(1, 1), next: &LFO_PARAMS},
    MenuItem{item: Parameter::GlobalLfo,  key: 'g', val_range: ValueRange::Int(1, 2), next: &LFO_PARAMS},
    MenuItem{item: Parameter::Aftertouch, key: 'a', val_range: ValueRange::Int(1, 1), next: &LFO_PARAMS},
    MenuItem{item: Parameter::PitchWheel, key: 'p', val_range: ValueRange::Int(1, 1), next: &LFO_PARAMS},
    MenuItem{item: Parameter::ModWheel,   key: 'm', val_range: ValueRange::Int(1, 1), next: &LFO_PARAMS},
];

pub static MOD_TARGETS: [MenuItem; 7] = [
    MenuItem{item: Parameter::Oscillator, key: 'o', val_range: ValueRange::Int(1, 3), next: &OSC_PARAMS},
    MenuItem{item: Parameter::Envelope,   key: 'e', val_range: ValueRange::Int(1, 2), next: &ENV_PARAMS},
    MenuItem{item: Parameter::Lfo,        key: 'l', val_range: ValueRange::Int(1, 2), next: &LFO_PARAMS},
    MenuItem{item: Parameter::GlobalLfo,  key: 'g', val_range: ValueRange::Int(1, 2), next: &LFO_PARAMS},
    MenuItem{item: Parameter::Filter,     key: 'f', val_range: ValueRange::Int(1, 2), next: &FILTER_PARAMS},
    MenuItem{item: Parameter::Delay,      key: 'd', val_range: ValueRange::Int(1, 1), next: &DELAY_PARAMS},
    MenuItem{item: Parameter::Modulation, key: 'm', val_range: ValueRange::Int(1, 16), next: &MOD_TARGET_PARAMS},
];

pub static MOD_TARGET_PARAMS: [MenuItem; 2] = [
    MenuItem{item: Parameter::Amount,    key: 'a', val_range: ValueRange::Float(0.0, 1.0, 0.01), next: &[]},
    MenuItem{item: Parameter::Active,    key: 'v', val_range: ValueRange::Int(0, 1),             next: &[]},
];

