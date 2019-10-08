use std::fmt::{self, Debug, Display};

#[derive(Clone, Copy, Debug)]
pub enum Parameter {
    // Function
    Oscillator,
    Filter,
    Amp,
    Lfo,
    Envelope,
    Mod,
    System,

    // Oscillator, Lfo
    Waveform,
    Frequency,
    Phase,

    // Filter
    Type,
    FilterFreq,
    Resonance,

    // Amp
    Volume,

    // Lfo

    // Envelope
    Attack,
    Decay,
    Sustain,
    Release,

    // Mod
    Source,
    Target,

    // Waveforms
    Sine,
    Square,
    Triangle
}

pub enum FunctionId {
    Int(u64),
    Index(usize),
    NoValue
}

#[derive(Clone, Copy, Debug)]
pub enum ParameterValue {
    Int(u64),
    Float(f32),
    Choice(usize),
    NoValue
}

impl fmt::Display for Parameter {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

pub struct SynthParam {
    pub function: Parameter,
    pub function_id: FunctionId,
    pub parameter: Parameter,
    pub value: ParameterValue
}

impl SynthParam {
    pub fn new(function: Parameter, function_id: FunctionId, parameter: Parameter, value: ParameterValue) -> Self {
        SynthParam{function, function_id, parameter, value}
    }
}
