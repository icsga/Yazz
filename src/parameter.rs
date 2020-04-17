use super::Float;

use std::fmt::{self, Debug, Display};

use serde::{Serialize, Deserialize};

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
    Phase,
    Blend,
    Level,
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
    pub fn new(function: Parameter, function_id: usize, parameter: Parameter) -> ParamId {
        ParamId{function, function_id, parameter}
    }

    pub fn set(&mut self, func: Parameter, func_id: usize, param: Parameter) {
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

impl fmt::Display for Parameter {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

#[derive(Debug)]
pub struct SynthParam {
    pub function: Parameter,
    pub function_id: usize,
    pub parameter: Parameter,
    pub value: ParameterValue
}

impl SynthParam {
    pub fn new(function: Parameter, function_id: usize, parameter: Parameter, value: ParameterValue) -> Self {
        SynthParam{function, function_id, parameter, value}
    }
}

/** Enum for ranges of valid values */
#[derive(Debug)]
pub enum ValueRange {
    IntRange(i64, i64),               // Range (min, max) of integer values
    FloatRange(Float, Float),         // Range (min, max) of float values
    ChoiceRange(&'static [MenuItem]), // A list of items to choose from
    FuncRange(&'static [MenuItem]),   // A list of (function-id) function entries
    ParamRange(&'static [MenuItem]),  // A list of (function-id-param) parameter entries
    NoRange
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
    pub mod_func: ModFunction,
    pub next: &'static [MenuItem]
}

/* Top-level menu */
pub static FUNCTIONS: [MenuItem; 7] = [
    MenuItem{item: Parameter::Oscillator, key: 'o', val_range: ValueRange::IntRange(1, 3),            mod_func: ModFunction::Source, next: &OSC_PARAMS},
    MenuItem{item: Parameter::Envelope,   key: 'e', val_range: ValueRange::IntRange(1, 2),            mod_func: ModFunction::Source, next: &ENV_PARAMS},
    MenuItem{item: Parameter::Lfo,        key: 'l', val_range: ValueRange::IntRange(1, 2),            mod_func: ModFunction::Source, next: &LFO_PARAMS},
    MenuItem{item: Parameter::GlobalLfo,  key: 'g', val_range: ValueRange::IntRange(1, 2),            mod_func: ModFunction::Source, next: &LFO_PARAMS},
    MenuItem{item: Parameter::Filter,     key: 'f', val_range: ValueRange::IntRange(1, 2),            mod_func: ModFunction::NoMod,  next: &FILTER_PARAMS},
    MenuItem{item: Parameter::Delay,      key: 'd', val_range: ValueRange::IntRange(1, 1),            mod_func: ModFunction::NoMod,  next: &DELAY_PARAMS},
    MenuItem{item: Parameter::Modulation, key: 'm', val_range: ValueRange::IntRange(1, 16),           mod_func: ModFunction::NoMod,  next: &MOD_PARAMS},
];

pub static OSC_PARAMS: [MenuItem; 10] = [
    MenuItem{item: Parameter::Waveform,  key: 'w', val_range: ValueRange::ChoiceRange(&OSC_WAVEFORM), mod_func: ModFunction::Target, next: &[]},
    MenuItem{item: Parameter::Level,     key: 'l', val_range: ValueRange::FloatRange(0.0, 100.0),     mod_func: ModFunction::Target, next: &[]},
    MenuItem{item: Parameter::Frequency, key: 'f', val_range: ValueRange::IntRange(-24, 24),          mod_func: ModFunction::Target, next: &[]},
    MenuItem{item: Parameter::Finetune,  key: 't', val_range: ValueRange::FloatRange(0.0, 1200.0),    mod_func: ModFunction::Target, next: &[]},
    MenuItem{item: Parameter::Blend,     key: 'b', val_range: ValueRange::FloatRange(0.0, 5.0),       mod_func: ModFunction::Target, next: &[]},
    MenuItem{item: Parameter::Phase,     key: 'p', val_range: ValueRange::FloatRange(0.0, 1.0),       mod_func: ModFunction::Target, next: &[]},
    MenuItem{item: Parameter::Sync,      key: 's', val_range: ValueRange::IntRange(0, 1),             mod_func: ModFunction::Target, next: &[]},
    MenuItem{item: Parameter::KeyFollow, key: 'k', val_range: ValueRange::IntRange(0, 1),             mod_func: ModFunction::Target, next: &[]},
    MenuItem{item: Parameter::Voices,    key: 'v', val_range: ValueRange::IntRange(1, 7),             mod_func: ModFunction::Target, next: &[]},
    MenuItem{item: Parameter::Spread,    key: 'e', val_range: ValueRange::FloatRange(0.0, 2.0),       mod_func: ModFunction::Target, next: &[]},
];

pub static LFO_PARAMS: [MenuItem; 2] = [
    MenuItem{item: Parameter::Waveform,  key: 'w', val_range: ValueRange::ChoiceRange(&LFO_WAVEFORM), mod_func: ModFunction::Target, next: &[]},
    MenuItem{item: Parameter::Frequency, key: 'f', val_range: ValueRange::FloatRange(0.0, 22000.0),   mod_func: ModFunction::Target, next: &[]},
    //MenuItem{item: Parameter::Phase,     key: 'p', val_range: ValueRange::FloatRange(0.0, 100.0),     mod_func: ModFunction::Target, next: &[]},
];

pub static FILTER_PARAMS: [MenuItem; 4] = [
    MenuItem{item: Parameter::Type,      key: 't', val_range: ValueRange::ChoiceRange(&FILTER_TYPE),   mod_func: ModFunction::NoMod,  next: &[]},
    MenuItem{item: Parameter::Cutoff,    key: 'c', val_range: ValueRange::FloatRange(1.0, 5000.0),    mod_func: ModFunction::Target, next: &[]},
    MenuItem{item: Parameter::Resonance, key: 'r', val_range: ValueRange::FloatRange(1.0, 100.0),       mod_func: ModFunction::Target, next: &[]},
    MenuItem{item: Parameter::Gain,      key: 'g', val_range: ValueRange::FloatRange(0.0, 1.0),       mod_func: ModFunction::Target, next: &[]},
];

pub static FILTER_TYPE: [MenuItem; 2] = [
    MenuItem{item: Parameter::RLPF,      key: 'l', val_range: ValueRange::NoRange,                    mod_func: ModFunction::NoMod,  next: &[]},
    MenuItem{item: Parameter::ResonZ,    key: 'r', val_range: ValueRange::NoRange,                    mod_func: ModFunction::NoMod,  next: &[]},
];

pub static ENV_PARAMS: [MenuItem; 5] = [
    MenuItem{item: Parameter::Attack,  key: 'a', val_range: ValueRange::FloatRange(0.0, 4000.0),      mod_func: ModFunction::Target, next: &[]}, // Value = Duration in ms
    MenuItem{item: Parameter::Decay,   key: 'd', val_range: ValueRange::FloatRange(0.0, 4000.0),      mod_func: ModFunction::Target, next: &[]},
    MenuItem{item: Parameter::Sustain, key: 's', val_range: ValueRange::FloatRange(0.0, 1.0),         mod_func: ModFunction::Target, next: &[]},
    MenuItem{item: Parameter::Release, key: 'r', val_range: ValueRange::FloatRange(0.0, 8000.0),      mod_func: ModFunction::Target, next: &[]},
    MenuItem{item: Parameter::Factor,  key: 'f', val_range: ValueRange::IntRange(1, 5),               mod_func: ModFunction::Target, next: &[]},
];

pub static OSC_WAVEFORM: [MenuItem; 5] = [
    MenuItem{item: Parameter::Sine,      key: 's', val_range: ValueRange::NoRange,                    mod_func: ModFunction::NoMod,  next: &[]},
    MenuItem{item: Parameter::Triangle,  key: 't', val_range: ValueRange::NoRange,                    mod_func: ModFunction::NoMod,  next: &[]},
    MenuItem{item: Parameter::Saw,       key: 'w', val_range: ValueRange::NoRange,                    mod_func: ModFunction::NoMod,  next: &[]},
    MenuItem{item: Parameter::Square,    key: 'q', val_range: ValueRange::NoRange,                    mod_func: ModFunction::NoMod,  next: &[]},
    MenuItem{item: Parameter::Noise ,    key: 'n', val_range: ValueRange::NoRange,                    mod_func: ModFunction::NoMod,  next: &[]},
];

pub static LFO_WAVEFORM: [MenuItem; 6] = [
    MenuItem{item: Parameter::Sine,      key: 's', val_range: ValueRange::NoRange,                    mod_func: ModFunction::NoMod,  next: &[]},
    MenuItem{item: Parameter::Triangle,  key: 't', val_range: ValueRange::NoRange,                    mod_func: ModFunction::NoMod,  next: &[]},
    MenuItem{item: Parameter::Saw,       key: 'w', val_range: ValueRange::NoRange,                    mod_func: ModFunction::NoMod,  next: &[]},
    MenuItem{item: Parameter::Square,    key: 'q', val_range: ValueRange::NoRange,                    mod_func: ModFunction::NoMod,  next: &[]},
    MenuItem{item: Parameter::Noise ,    key: 'n', val_range: ValueRange::NoRange,                    mod_func: ModFunction::NoMod,  next: &[]},
    MenuItem{item: Parameter::SampleHold,key: 'h', val_range: ValueRange::NoRange,                    mod_func: ModFunction::NoMod,  next: &[]},
];

pub static DELAY_PARAMS: [MenuItem; 4] = [
    MenuItem{item: Parameter::Time,      key: 't', val_range: ValueRange::FloatRange(0.01, 1.0),      mod_func: ModFunction::Target, next: &[]},
    MenuItem{item: Parameter::Level,     key: 'l', val_range: ValueRange::FloatRange(0.0, 1.0),       mod_func: ModFunction::Target, next: &[]},
    MenuItem{item: Parameter::Feedback,  key: 'f', val_range: ValueRange::FloatRange(0.0, 1.0),       mod_func: ModFunction::Target, next: &[]},
    MenuItem{item: Parameter::Tone,      key: 'o', val_range: ValueRange::FloatRange(100.0, 5000.0),  mod_func: ModFunction::Target, next: &[]},
];

pub static MOD_PARAMS: [MenuItem; 4] = [
    MenuItem{item: Parameter::Source,    key: 's', val_range: ValueRange::FuncRange(&MOD_SOURCES),    mod_func: ModFunction::NoMod,  next: &MOD_SOURCES},
    MenuItem{item: Parameter::Target,    key: 't', val_range: ValueRange::ParamRange(&MOD_TARGETS),   mod_func: ModFunction::NoMod,  next: &MOD_TARGETS},
    MenuItem{item: Parameter::Amount,    key: 'a', val_range: ValueRange::FloatRange(0.0, 1.0),       mod_func: ModFunction::NoMod,  next: &[]},
    MenuItem{item: Parameter::Active,    key: 'v', val_range: ValueRange::IntRange(0, 1),             mod_func: ModFunction::NoMod,  next: &[]},
];

pub static MOD_SOURCES: [MenuItem; 4] = [
    MenuItem{item: Parameter::Oscillator, key: 'o', val_range: ValueRange::IntRange(1, 3),            mod_func: ModFunction::Source, next: &OSC_PARAMS},
    MenuItem{item: Parameter::Envelope,   key: 'e', val_range: ValueRange::IntRange(1, 2),            mod_func: ModFunction::Source, next: &ENV_PARAMS},
    MenuItem{item: Parameter::Lfo,        key: 'l', val_range: ValueRange::IntRange(1, 2),            mod_func: ModFunction::Source, next: &LFO_PARAMS},
    MenuItem{item: Parameter::GlobalLfo,  key: 'g', val_range: ValueRange::IntRange(1, 2),            mod_func: ModFunction::Source, next: &LFO_PARAMS},
];

pub static MOD_TARGETS: [MenuItem; 6] = [
    MenuItem{item: Parameter::Oscillator, key: 'o', val_range: ValueRange::IntRange(1, 3),            mod_func: ModFunction::Source, next: &OSC_PARAMS},
    MenuItem{item: Parameter::Envelope,   key: 'e', val_range: ValueRange::IntRange(1, 2),            mod_func: ModFunction::Source, next: &ENV_PARAMS},
    MenuItem{item: Parameter::Lfo,        key: 'l', val_range: ValueRange::IntRange(1, 2),            mod_func: ModFunction::Source, next: &LFO_PARAMS},
    MenuItem{item: Parameter::GlobalLfo,  key: 'g', val_range: ValueRange::IntRange(1, 2),            mod_func: ModFunction::Source, next: &LFO_PARAMS},
    MenuItem{item: Parameter::Filter,     key: 'f', val_range: ValueRange::IntRange(1, 2),            mod_func: ModFunction::NoMod,  next: &FILTER_PARAMS},
    MenuItem{item: Parameter::Delay,      key: 'd', val_range: ValueRange::IntRange(1, 1),            mod_func: ModFunction::NoMod,  next: &DELAY_PARAMS},
];

