use super::Float;
use super::value_range::ValueRange;
use super::synth::*;
use super::voice::*;

use std::fmt::{self, Debug};

use serde::{Serialize, Deserialize};

#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub enum Parameter {
    None,

    // Function
    Oscillator,
    Filter,
    Amp,
    Lfo,
    GlobalLfo,
    Envelope,
    Modulation,
    Delay,
    Patch,
    System,

    // Oscillator, Lfo
    Waveform,
    Level,
    Wavetable,
    WaveIndex,
    Frequency,
    Tune,
    Finetune,
    Sync,
    KeyFollow,
    Routing,
    Voices,
    Spread,
    VelSens,
    EnvDepth,
    Phase,

    // Oscillator routing
    Filter1,
    Filter2,
    Direct,

    // Filter
    Type,
    Cutoff,
    Q,
    Resonance,
    Gain,
    Aux,
    // Filter types
    RLPF,
    ResonZ,
    Moog,
    OnePole,
    SEM_LPF,
    SEM_BPF,
    SEM_HPF,
    SEM_BSF,
    K35_LPF,
    K35_HPF,
    OM_LPF,
    OM_BPF,
    OM_HPF,

    // Amp
    Volume,
    Drive,

    // Lfo

    // Envelope
    Attack,
    Decay,
    Sustain,
    Release,
    Factor,
    Loop,
    ResetToZero,

    // Mod
    Source,
    Target,
    Amount,
    Active,

    // Waveforms
    Sine,
    Triangle,
    Saw,
    SawDown,
    Square,
    Noise,
    SampleHold,

    // Delay
    Time,
    Feedback,
    Tone,
    // Delay types
    Stereo,
    PingPong,

    // MIDI parameters
    KeyValue,
    Velocity,
    KeyAftertouch,
    Aftertouch,
    Pitchbend,
    ModWheel,
    SustainPedal,

    // System parameters
    Idle,
    Busy,
    PlayMode,
    Poly,
    Mono,
    Legato,
    FilterRouting,
    Parallel,
    Serial,
    Bpm,
    Allocation,
    PanOrigin,

    // Voice allocation types
    Ascending,
    RoundRobin,
    Random,

    // Pan origin
    Center,
    Left,
    Right,

    // Sync values
    Off,
    Whole,
    DottedHalf,
    Half,
    DottedQuarter,
    Quarter,
    DottedEigth,
    Eigth,
    Sixteenth,
}
#[warn(non_camel_case_types)]

impl Default for Parameter {
    fn default() -> Self { Parameter::Oscillator }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct FunctionId {
    pub function: Parameter,
    pub function_id: usize,
}

/** Identifies a synth parameter. */
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

/** List of possible value types of a synth parameter. */
#[derive(Clone, Copy, Debug)]
pub enum ParameterValue {
    /// Integer value
    Int(i64),
    /// Float value
    Float(Float),
    /// Index into a static list
    Choice(usize),
    /// Index into a dynamic list
    Dynamic(Parameter, usize),
    /// Value is itself a function ID (e.g. modulation source)
    Function(FunctionId),
    /// Value is itself a parameter ID (e.g. modulation target)
    Param(ParamId),
    /// No value
    NoValue
}

impl Default for ParameterValue {
    fn default() -> Self { ParameterValue::NoValue }
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
            ParameterValue::Int(_) => {*self = ParameterValue::Int(value as i64)},
            ParameterValue::Float(_) => {*self = ParameterValue::Float(value)},
            ParameterValue::Choice(_) => {*self = ParameterValue::Choice(value as usize)},
            _ => panic!("Cannot convert parameter value to float")
        }
    }
}

impl fmt::Display for Parameter {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

#[derive(Clone, Copy, Debug, Default)]
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

    pub fn set(&mut self,
               func: Parameter,
               func_id: usize,
               param: Parameter,
               value: ParameterValue) {
        self.function = func;
        self.function_id = func_id;
        self.parameter = param;
        self.value = value;
    }

    pub fn equals(&self, other: &ParamId) -> bool {
        self.function == other.function &&
        self.function_id == other.function_id &&
        self.parameter == other.parameter
    }
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
        for s in item_list {
            if s.item == item {
                return &s;
            }
        }
        panic!();
    }

    pub fn get_item_index(item: Parameter, item_list: &'static [MenuItem]) -> usize {
        for (i, s) in item_list.iter().enumerate() {
            if s.item == item {
                return i;
            }
        }
        panic!("Unable to find item {}", item);
    }
}

/* Top-level menu */
pub static FUNCTIONS: [MenuItem; 8] = [
    MenuItem{item: Parameter::Oscillator, key: 'o', val_range: ValueRange::Int(1, NUM_OSCILLATORS as i64),  next: &OSC_PARAMS},
    MenuItem{item: Parameter::Envelope,   key: 'e', val_range: ValueRange::Int(1, NUM_ENVELOPES as i64),    next: &ENV_PARAMS},
    MenuItem{item: Parameter::Lfo,        key: 'l', val_range: ValueRange::Int(1, NUM_LFOS as i64),         next: &LFO_PARAMS},
    MenuItem{item: Parameter::GlobalLfo,  key: 'g', val_range: ValueRange::Int(1, NUM_GLOBAL_LFOS as i64),  next: &LFO_PARAMS},
    MenuItem{item: Parameter::Filter,     key: 'f', val_range: ValueRange::Int(1, NUM_FILTERS as i64),      next: &FILTER_PARAMS},
    MenuItem{item: Parameter::Delay,      key: 'd', val_range: ValueRange::Int(1, 1),                       next: &DELAY_PARAMS},
    MenuItem{item: Parameter::Modulation, key: 'm', val_range: ValueRange::Int(1, NUM_MODULATORS as i64),   next: &MOD_PARAMS},
    MenuItem{item: Parameter::Patch,      key: 'p', val_range: ValueRange::Int(1, 1),                       next: &PATCH_PARAMS},
];

pub static OSC_PARAMS: [MenuItem; 11] = [
    MenuItem{item: Parameter::Level,     key: 'l', val_range: ValueRange::Float(0.0, 100.0, 1.0),       next: &[]},
    MenuItem{item: Parameter::Tune,      key: 't', val_range: ValueRange::Int(-24, 24),                 next: &[]},
    MenuItem{item: Parameter::Finetune,  key: 'f', val_range: ValueRange::Float(-100.0, 100.0, 1.0),    next: &[]},
    MenuItem{item: Parameter::Sync,      key: 's', val_range: ValueRange::Int(0, 1),                    next: &[]},
    MenuItem{item: Parameter::KeyFollow, key: 'k', val_range: ValueRange::Int(0, 1),                    next: &[]},
    MenuItem{item: Parameter::Routing,   key: 'r', val_range: ValueRange::Choice(&OSC_ROUTING),         next: &[]},
    MenuItem{item: Parameter::Type,      key: 'y', val_range: ValueRange::Choice(&OSC_TYPES),           next: &[]},

    MenuItem{item: Parameter::Wavetable, key: 'w', val_range: ValueRange::Dynamic(Parameter::Wavetable),next: &[]},
    MenuItem{item: Parameter::WaveIndex, key: 'i', val_range: ValueRange::Float(0.0, 1.0, 0.01),        next: &[]},
    MenuItem{item: Parameter::Voices,    key: 'v', val_range: ValueRange::Int(1, 7),                    next: &[]},
    MenuItem{item: Parameter::Spread,    key: 'e', val_range: ValueRange::Float(0.0, 2.0, 0.01),        next: &[]},
];

pub static OSC_ROUTING: [MenuItem; 3] = [
    MenuItem{item: Parameter::Filter1, key: '1', val_range: ValueRange::NoRange, next: &[]},
    MenuItem{item: Parameter::Filter2, key: '2', val_range: ValueRange::NoRange, next: &[]},
    MenuItem{item: Parameter::Direct,  key: 'd', val_range: ValueRange::NoRange, next: &[]},
];

pub static OSC_TYPES: [MenuItem; 2] = [
    MenuItem{item: Parameter::Wavetable, key: 'w', val_range: ValueRange::NoRange, next: &[]},
    MenuItem{item: Parameter::Noise,     key: 'n', val_range: ValueRange::NoRange, next: &[]},
];

pub static LFO_PARAMS: [MenuItem; 5] = [
    MenuItem{item: Parameter::Waveform,  key: 'w', val_range: ValueRange::Choice(&LFO_WAVEFORM), next: &[]},
    MenuItem{item: Parameter::Frequency, key: 'f', val_range: ValueRange::Float(0.0, 44.1, 0.1), next: &[]},
    MenuItem{item: Parameter::Sync,      key: 's', val_range: ValueRange::Choice(&SYNC_OPTIONS), next: &[]},
    MenuItem{item: Parameter::Phase,     key: 'p', val_range: ValueRange::Float(0.0, 1.0, 0.01), next: &[]},
    MenuItem{item: Parameter::Amount,    key: 'a', val_range: ValueRange::Float(0.0, 1.0, 0.01), next: &[]},
];

pub static FILTER_PARAMS: [MenuItem; 7] = [
    MenuItem{item: Parameter::Type,      key: 't', val_range: ValueRange::Choice(&FILTER_TYPE),    next: &[]},
    MenuItem{item: Parameter::Cutoff,    key: 'c', val_range: ValueRange::Float(1.0, 8000.0, 20.0), next: &[]},
    MenuItem{item: Parameter::Resonance, key: 'r', val_range: ValueRange::Float(0.0, 1.0, 0.01),   next: &[]},
    MenuItem{item: Parameter::Gain,      key: 'g', val_range: ValueRange::Float(0.0, 2.0, 0.01),   next: &[]},
    MenuItem{item: Parameter::Aux,       key: 'a', val_range: ValueRange::Float(0.0, 1.0, 0.01),   next: &[]},
    MenuItem{item: Parameter::EnvDepth,  key: 'e', val_range: ValueRange::Float(0.0, 1.0, 0.01),   next: &[]},
    MenuItem{item: Parameter::KeyFollow, key: 'k', val_range: ValueRange::Int(0, 1),               next: &[]},
];

pub static FILTER_TYPE: [MenuItem; 10] = [
    MenuItem{item: Parameter::None,      key: 'n', val_range: ValueRange::NoRange, next: &[]},
    MenuItem{item: Parameter::SEM_LPF,   key: 's', val_range: ValueRange::NoRange, next: &[]},
    MenuItem{item: Parameter::SEM_BPF,   key: 'b', val_range: ValueRange::NoRange, next: &[]},
    MenuItem{item: Parameter::SEM_HPF,   key: 'h', val_range: ValueRange::NoRange, next: &[]},
    MenuItem{item: Parameter::SEM_BSF,   key: 'o', val_range: ValueRange::NoRange, next: &[]},
    MenuItem{item: Parameter::K35_LPF,   key: 'k', val_range: ValueRange::NoRange, next: &[]},
    MenuItem{item: Parameter::K35_HPF,   key: 'p', val_range: ValueRange::NoRange, next: &[]},
    MenuItem{item: Parameter::OM_LPF,    key: 'm', val_range: ValueRange::NoRange, next: &[]},
    MenuItem{item: Parameter::OM_BPF,    key: 'g', val_range: ValueRange::NoRange, next: &[]},
    MenuItem{item: Parameter::OM_HPF,    key: 'f', val_range: ValueRange::NoRange, next: &[]},
];

pub static ENV_PARAMS: [MenuItem; 8] = [
    MenuItem{item: Parameter::Attack,     key: 'a', val_range: ValueRange::Float(1.0, 4000.0, 1.0), next: &[]}, // Value = Duration in ms
    MenuItem{item: Parameter::Decay,      key: 'd', val_range: ValueRange::Float(1.0, 4000.0, 1.0), next: &[]},
    MenuItem{item: Parameter::Sustain,    key: 's', val_range: ValueRange::Float(0.0, 1.0, 0.001),  next: &[]},
    MenuItem{item: Parameter::Release,    key: 'r', val_range: ValueRange::Float(1.0, 8000.0, 1.0), next: &[]},
    MenuItem{item: Parameter::Factor,     key: 'f', val_range: ValueRange::Int(1, 5),               next: &[]},
    MenuItem{item: Parameter::Delay,      key: 'e', val_range: ValueRange::Float(0.0, 4000.0, 1.0), next: &[]},
    MenuItem{item: Parameter::Loop,       key: 'l', val_range: ValueRange::Int(0, 1),               next: &[]},
    MenuItem{item: Parameter::ResetToZero,key: 'z', val_range: ValueRange::Int(0, 1),               next: &[]},
];

pub static LFO_WAVEFORM: [MenuItem; 7] = [
    MenuItem{item: Parameter::Sine,      key: 's', val_range: ValueRange::NoRange, next: &[]},
    MenuItem{item: Parameter::Triangle,  key: 't', val_range: ValueRange::NoRange, next: &[]},
    MenuItem{item: Parameter::Saw,       key: 'w', val_range: ValueRange::NoRange, next: &[]},
    MenuItem{item: Parameter::SawDown,   key: 'd', val_range: ValueRange::NoRange, next: &[]},
    MenuItem{item: Parameter::Square,    key: 'q', val_range: ValueRange::NoRange, next: &[]},
    MenuItem{item: Parameter::SampleHold,key: 'h', val_range: ValueRange::NoRange, next: &[]},
    MenuItem{item: Parameter::Noise ,    key: 'n', val_range: ValueRange::NoRange, next: &[]},
];

pub static SYNC_OPTIONS: [MenuItem; 9] = [
    MenuItem{item: Parameter::Off,          key: 'o', val_range: ValueRange::NoRange, next: &[]},
    MenuItem{item: Parameter::Whole,        key: 'w', val_range: ValueRange::NoRange, next: &[]},
    MenuItem{item: Parameter::DottedHalf,   key: 't', val_range: ValueRange::NoRange, next: &[]},
    MenuItem{item: Parameter::Half,         key: 'h', val_range: ValueRange::NoRange, next: &[]},
    MenuItem{item: Parameter::DottedQuarter,key: 'u', val_range: ValueRange::NoRange, next: &[]},
    MenuItem{item: Parameter::Quarter,      key: 'q', val_range: ValueRange::NoRange, next: &[]},
    MenuItem{item: Parameter::DottedEigth,  key: 'd', val_range: ValueRange::NoRange, next: &[]},
    MenuItem{item: Parameter::Eigth,        key: 'e', val_range: ValueRange::NoRange, next: &[]},
    MenuItem{item: Parameter::Sixteenth,    key: 's', val_range: ValueRange::NoRange, next: &[]},
];

pub static DELAY_PARAMS: [MenuItem; 6] = [
    MenuItem{item: Parameter::Time,      key: 't', val_range: ValueRange::Float(0.01, 1.0, 0.01),    next: &[]},
    MenuItem{item: Parameter::Sync,      key: 's', val_range: ValueRange::Choice(&SYNC_OPTIONS),     next: &[]},
    MenuItem{item: Parameter::Level,     key: 'l', val_range: ValueRange::Float(0.0, 1.0, 0.01),     next: &[]},
    MenuItem{item: Parameter::Feedback,  key: 'f', val_range: ValueRange::Float(0.0, 1.0, 0.01),     next: &[]},
    MenuItem{item: Parameter::Tone,      key: 'o', val_range: ValueRange::Float(100.0, 5000.0, 1.0), next: &[]},
    MenuItem{item: Parameter::Type,      key: 'y', val_range: ValueRange::Choice(&DELAY_TYPE),       next: &[]},
];

pub static DELAY_TYPE: [MenuItem; 2] = [
    MenuItem{item: Parameter::Stereo,    key: 's', val_range: ValueRange::NoRange, next: &[]},
    MenuItem{item: Parameter::PingPong,  key: 'p', val_range: ValueRange::NoRange, next: &[]},
];

pub static MOD_PARAMS: [MenuItem; 4] = [
    MenuItem{item: Parameter::Source,    key: 's', val_range: ValueRange::Func(&MOD_SOURCES),    next: &MOD_SOURCES},
    MenuItem{item: Parameter::Target,    key: 't', val_range: ValueRange::Param(&MOD_TARGETS),   next: &MOD_TARGETS},
    MenuItem{item: Parameter::Amount,    key: 'a', val_range: ValueRange::Float(-1.0, 1.0, 0.01),next: &[]},
    MenuItem{item: Parameter::Active,    key: 'v', val_range: ValueRange::Int(0, 1),             next: &[]},
];

pub static PATCH_PARAMS: [MenuItem; 12] = [
    MenuItem{item: Parameter::Level,        key: 'l', val_range: ValueRange::Float(0.0, 100.0, 1.0),    next: &[]},
    MenuItem{item: Parameter::Drive,        key: 'd', val_range: ValueRange::Float(0.0, 10.0, 1.0),     next: &[]},
    MenuItem{item: Parameter::Pitchbend ,   key: 'p', val_range: ValueRange::Int(0, 12),                next: &[]},
    MenuItem{item: Parameter::VelSens,      key: 'v', val_range: ValueRange::Float(0.0, 1.0, 0.01),     next: &[]},
    MenuItem{item: Parameter::EnvDepth,     key: 'e', val_range: ValueRange::Float(0.0, 1.0, 0.01),     next: &[]},
    MenuItem{item: Parameter::PlayMode,     key: 'm', val_range: ValueRange::Choice(&PLAY_MODES),       next: &[]},
    MenuItem{item: Parameter::FilterRouting,key: 'f', val_range: ValueRange::Choice(&FILTER_ROUTING),   next: &[]},
    MenuItem{item: Parameter::Bpm,          key: 'b', val_range: ValueRange::Float(1.0, 240.0, 1.0),    next: &[]},
    MenuItem{item: Parameter::Voices,       key: 'n', val_range: ValueRange::Int(1, NUM_VOICES as i64), next: &[]},
    MenuItem{item: Parameter::Spread,       key: 's', val_range: ValueRange::Float(0.0, 1.0, 0.01),     next: &[]},
    MenuItem{item: Parameter::Allocation,   key: 'a', val_range: ValueRange::Choice(&VOICE_ALLOCATION), next: &[]},
    MenuItem{item: Parameter::PanOrigin,    key: 'o', val_range: ValueRange::Choice(&PAN_ORIGIN),       next: &[]},
];

pub static MOD_SOURCES: [MenuItem; 9] = [
    MenuItem{item: Parameter::Oscillator,  key: 'o', val_range: ValueRange::Int(1, 3), next: &OSC_PARAMS},
    MenuItem{item: Parameter::Envelope,    key: 'e', val_range: ValueRange::Int(1, 3), next: &ENV_PARAMS},
    MenuItem{item: Parameter::Lfo,         key: 'l', val_range: ValueRange::Int(1, 2), next: &LFO_PARAMS},
    MenuItem{item: Parameter::Velocity,    key: 'v', val_range: ValueRange::Int(1, 1), next: &LFO_PARAMS},
    MenuItem{item: Parameter::GlobalLfo,   key: 'g', val_range: ValueRange::Int(1, 2), next: &LFO_PARAMS},
    MenuItem{item: Parameter::Aftertouch,  key: 'a', val_range: ValueRange::Int(1, 1), next: &LFO_PARAMS},
    MenuItem{item: Parameter::Pitchbend,   key: 'p', val_range: ValueRange::Int(1, 1), next: &LFO_PARAMS},
    MenuItem{item: Parameter::ModWheel,    key: 'm', val_range: ValueRange::Int(1, 1), next: &LFO_PARAMS},
    MenuItem{item: Parameter::SustainPedal,key: 's', val_range: ValueRange::Int(1, 1), next: &LFO_PARAMS},
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

pub static PLAY_MODES: [MenuItem; 3] = [
    MenuItem{item: Parameter::Poly,      key: 'p', val_range: ValueRange::NoRange, next: &[]},
    MenuItem{item: Parameter::Mono,      key: 'm', val_range: ValueRange::NoRange, next: &[]},
    MenuItem{item: Parameter::Legato,    key: 'l', val_range: ValueRange::NoRange, next: &[]},
];

pub static FILTER_ROUTING: [MenuItem; 2] = [
    MenuItem{item: Parameter::Parallel, key: 'p', val_range: ValueRange::NoRange, next: &[]},
    MenuItem{item: Parameter::Serial,   key: 's', val_range: ValueRange::NoRange, next: &[]},
];

pub static VOICE_ALLOCATION: [MenuItem; 3] = [
    MenuItem{item: Parameter::RoundRobin, key: 'r', val_range: ValueRange::NoRange, next: &[]},
    MenuItem{item: Parameter::Ascending,  key: 'a', val_range: ValueRange::NoRange, next: &[]},
    MenuItem{item: Parameter::Random,     key: 'o', val_range: ValueRange::NoRange, next: &[]},
];

pub static PAN_ORIGIN: [MenuItem; 3] = [
    MenuItem{item: Parameter::Center, key: 'c', val_range: ValueRange::NoRange, next: &[]},
    MenuItem{item: Parameter::Left,   key: 'l', val_range: ValueRange::NoRange, next: &[]},
    MenuItem{item: Parameter::Right,  key: 'r', val_range: ValueRange::NoRange, next: &[]},
];

