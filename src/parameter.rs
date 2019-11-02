use super::Float;

use std::fmt::{self, Debug, Display};

use serde::{Serialize, Deserialize};

#[derive(PartialEq, Clone, Copy, Debug, Serialize, Deserialize)]
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
    Sync,
    KeyFollow,
    Voices,
    Spread,

    // Filter
    Type,
    Cutoff,
    Resonance,

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

    // MIDI parameters
    KeyValue,
    KeyAttack,
    Aftertouch
}

impl Default for Parameter {
    fn default() -> Self { Parameter::Oscillator }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct FunctionId {
    pub function: Parameter,
    pub function_id: usize,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ParamId {
    pub function: Parameter,
    pub function_id: usize,
    pub parameter: Parameter
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

#[derive(Debug)]
pub enum ValueRange {
    IntRange(i64, i64),
    FloatRange(Float, Float),
    ChoiceRange(&'static [Selection]),
    ModRange(&'static [Selection]),
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
pub struct Selection {
    pub item: Parameter,
    pub key: char,
    pub val_range: ValueRange,
    pub mod_func: ModFunction,
    pub next: &'static [Selection]
}

/* Top-level menu */
pub static FUNCTIONS: [Selection; 7] = [
    Selection{item: Parameter::Oscillator, key: 'o', val_range: ValueRange::IntRange(1, 3),            mod_func: ModFunction::Source, next: &OSC_PARAMS},
    Selection{item: Parameter::Envelope,   key: 'e', val_range: ValueRange::IntRange(1, 2),            mod_func: ModFunction::Source, next: &ENV_PARAMS},
    Selection{item: Parameter::Lfo,        key: 'l', val_range: ValueRange::IntRange(1, 2),            mod_func: ModFunction::Source, next: &LFO_PARAMS},
    Selection{item: Parameter::GlobalLfo,  key: 'g', val_range: ValueRange::IntRange(1, 2),            mod_func: ModFunction::Source, next: &LFO_PARAMS},
    Selection{item: Parameter::Filter,     key: 'f', val_range: ValueRange::IntRange(1, 2),            mod_func: ModFunction::NoMod,  next: &FILTER_PARAMS},
    Selection{item: Parameter::Delay,      key: 'd', val_range: ValueRange::IntRange(1, 1),            mod_func: ModFunction::NoMod,  next: &DELAY_PARAMS},
    Selection{item: Parameter::Modulation, key: 'm', val_range: ValueRange::IntRange(1, 16),           mod_func: ModFunction::NoMod,  next: &MOD_PARAMS},
];

pub static OSC_PARAMS: [Selection; 9] = [
    Selection{item: Parameter::Waveform,  key: 'w', val_range: ValueRange::ChoiceRange(&OSC_WAVEFORM), mod_func: ModFunction::Target, next: &[]},
    Selection{item: Parameter::Level,     key: 'l', val_range: ValueRange::FloatRange(0.0, 100.0),     mod_func: ModFunction::Target, next: &[]},
    Selection{item: Parameter::Frequency, key: 'f', val_range: ValueRange::IntRange(-24, 24),          mod_func: ModFunction::Target, next: &[]},
    Selection{item: Parameter::Blend,     key: 'b', val_range: ValueRange::FloatRange(0.0, 5.0),       mod_func: ModFunction::Target, next: &[]},
    Selection{item: Parameter::Phase,     key: 'p', val_range: ValueRange::FloatRange(0.0, 1.0),       mod_func: ModFunction::Target, next: &[]},
    Selection{item: Parameter::Sync,      key: 's', val_range: ValueRange::IntRange(0, 1),             mod_func: ModFunction::Target, next: &[]},
    Selection{item: Parameter::KeyFollow, key: 'k', val_range: ValueRange::IntRange(0, 1),             mod_func: ModFunction::Target, next: &[]},
    Selection{item: Parameter::Voices,    key: 'v', val_range: ValueRange::IntRange(1, 7),             mod_func: ModFunction::Target, next: &[]},
    Selection{item: Parameter::Spread,    key: 'e', val_range: ValueRange::FloatRange(0.0, 2.0),       mod_func: ModFunction::Target, next: &[]},
];

pub static LFO_PARAMS: [Selection; 3] = [
    Selection{item: Parameter::Waveform,  key: 'w', val_range: ValueRange::ChoiceRange(&LFO_WAVEFORM), mod_func: ModFunction::Target, next: &[]},
    Selection{item: Parameter::Frequency, key: 'f', val_range: ValueRange::FloatRange(0.0, 22000.0),   mod_func: ModFunction::Target, next: &[]},
    Selection{item: Parameter::Phase,     key: 'p', val_range: ValueRange::FloatRange(0.0, 100.0),     mod_func: ModFunction::Target, next: &[]},
];

pub static FILTER_PARAMS: [Selection; 3] = [
    Selection{item: Parameter::Type,      key: 't', val_range: ValueRange::IntRange(1, 3),             mod_func: ModFunction::NoMod,  next: &[]},
    Selection{item: Parameter::Cutoff,    key: 'c', val_range: ValueRange::FloatRange(0.0, 22000.0),   mod_func: ModFunction::Target, next: &[]},
    Selection{item: Parameter::Resonance, key: 'r', val_range: ValueRange::FloatRange(0.0, 100.0),     mod_func: ModFunction::Target, next: &[]},
];

pub static ENV_PARAMS: [Selection; 5] = [
    Selection{item: Parameter::Attack,  key: 'a', val_range: ValueRange::FloatRange(0.0, 4000.0),      mod_func: ModFunction::Target, next: &[]}, // Value = Duration in ms
    Selection{item: Parameter::Decay,   key: 'd', val_range: ValueRange::FloatRange(0.0, 4000.0),      mod_func: ModFunction::Target, next: &[]},
    Selection{item: Parameter::Sustain, key: 's', val_range: ValueRange::FloatRange(0.0, 100.0),       mod_func: ModFunction::Target, next: &[]},
    Selection{item: Parameter::Release, key: 'r', val_range: ValueRange::FloatRange(0.0, 8000.0),      mod_func: ModFunction::Target, next: &[]},
    Selection{item: Parameter::Factor,  key: 'f', val_range: ValueRange::IntRange(1, 5),               mod_func: ModFunction::Target, next: &[]},
];

pub static OSC_WAVEFORM: [Selection; 5] = [
    Selection{item: Parameter::Sine,      key: 's', val_range: ValueRange::NoRange,                    mod_func: ModFunction::NoMod,  next: &[]},
    Selection{item: Parameter::Triangle,  key: 't', val_range: ValueRange::NoRange,                    mod_func: ModFunction::NoMod,  next: &[]},
    Selection{item: Parameter::Saw,       key: 'w', val_range: ValueRange::NoRange,                    mod_func: ModFunction::NoMod,  next: &[]},
    Selection{item: Parameter::Square,    key: 'q', val_range: ValueRange::NoRange,                    mod_func: ModFunction::NoMod,  next: &[]},
    Selection{item: Parameter::Noise ,    key: 'n', val_range: ValueRange::NoRange,                    mod_func: ModFunction::NoMod,  next: &[]},
];

pub static LFO_WAVEFORM: [Selection; 5] = [
    Selection{item: Parameter::Sine,      key: 's', val_range: ValueRange::NoRange,                    mod_func: ModFunction::NoMod,  next: &[]},
    Selection{item: Parameter::Triangle,  key: 't', val_range: ValueRange::NoRange,                    mod_func: ModFunction::NoMod,  next: &[]},
    Selection{item: Parameter::Saw,       key: 'w', val_range: ValueRange::NoRange,                    mod_func: ModFunction::NoMod,  next: &[]},
    Selection{item: Parameter::Square,    key: 'q', val_range: ValueRange::NoRange,                    mod_func: ModFunction::NoMod,  next: &[]},
    Selection{item: Parameter::SampleHold,key: 'h', val_range: ValueRange::NoRange,                    mod_func: ModFunction::NoMod,  next: &[]},
];

pub static DELAY_PARAMS: [Selection; 3] = [
    Selection{item: Parameter::Time,      key: 't', val_range: ValueRange::FloatRange(0.01, 1.0),      mod_func: ModFunction::Target, next: &[]},
    Selection{item: Parameter::Level,     key: 'l', val_range: ValueRange::FloatRange(0.0, 1.0),       mod_func: ModFunction::Target, next: &[]},
    Selection{item: Parameter::Feedback,  key: 'f', val_range: ValueRange::FloatRange(0.0, 1.0),       mod_func: ModFunction::Target, next: &[]},
];

pub static MOD_PARAMS: [Selection; 4] = [
    Selection{item: Parameter::Source,    key: 's', val_range: ValueRange::ModRange(&MOD_SOURCES),     mod_func: ModFunction::NoMod,  next: &[]},
    Selection{item: Parameter::Target,    key: 't', val_range: ValueRange::ModRange(&MOD_TARGETS),     mod_func: ModFunction::NoMod,  next: &[]},
    Selection{item: Parameter::Amount,    key: 'a', val_range: ValueRange::FloatRange(0.0, 1.0),       mod_func: ModFunction::NoMod,  next: &[]},
    Selection{item: Parameter::Active,    key: 'v', val_range: ValueRange::IntRange(0, 1),             mod_func: ModFunction::NoMod,  next: &[]},
];

pub static MOD_SOURCES: [Selection; 4] = [
    Selection{item: Parameter::Oscillator, key: 'o', val_range: ValueRange::IntRange(1, 3),            mod_func: ModFunction::Source, next: &OSC_PARAMS},
    Selection{item: Parameter::Envelope,   key: 'e', val_range: ValueRange::IntRange(1, 2),            mod_func: ModFunction::Source, next: &ENV_PARAMS},
    Selection{item: Parameter::Lfo,        key: 'l', val_range: ValueRange::IntRange(1, 2),            mod_func: ModFunction::Source, next: &LFO_PARAMS},
    Selection{item: Parameter::GlobalLfo,  key: 'g', val_range: ValueRange::IntRange(1, 2),            mod_func: ModFunction::Source, next: &LFO_PARAMS},
];

pub static MOD_TARGETS: [Selection; 6] = [
    Selection{item: Parameter::Oscillator, key: 'o', val_range: ValueRange::IntRange(1, 3),            mod_func: ModFunction::Source, next: &OSC_PARAMS},
    Selection{item: Parameter::Envelope,   key: 'e', val_range: ValueRange::IntRange(1, 2),            mod_func: ModFunction::Source, next: &ENV_PARAMS},
    Selection{item: Parameter::Lfo,        key: 'l', val_range: ValueRange::IntRange(1, 2),            mod_func: ModFunction::Source, next: &LFO_PARAMS},
    Selection{item: Parameter::GlobalLfo,  key: 'g', val_range: ValueRange::IntRange(1, 2),            mod_func: ModFunction::Source, next: &LFO_PARAMS},
    Selection{item: Parameter::Filter,     key: 'f', val_range: ValueRange::IntRange(1, 2),            mod_func: ModFunction::NoMod,  next: &FILTER_PARAMS},
    Selection{item: Parameter::Delay,      key: 'd', val_range: ValueRange::IntRange(1, 1),            mod_func: ModFunction::NoMod,  next: &DELAY_PARAMS},
];

#[derive(Debug)]
pub struct SelectedItem {
    pub item_list: &'static [Selection], // The selection this item is coming from
    pub item_index: usize, // Index into the selection list
    pub value: ParameterValue, // ID or value of the selected item
}

