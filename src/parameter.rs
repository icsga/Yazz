use std::fmt::{self, Debug, Display};

#[derive(Debug)]
pub enum Function {
    Oscillator,
    Filter,
    Amp,
    Lfo,
    Envelope,
    Mod,
    System
}

pub enum FunctionId {
    Int(u32),
    Index(usize),
    NoValue
}

pub enum Parameter {
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
    Target
}

pub enum ParameterValue {
    Int(u64),
    Float(f32),
    Index(usize),
    NoValue
}

impl fmt::Display for Function {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

pub struct SynthParam {
    function: Function,
    function_id: FunctionId,
    parameter: Parameter,
    param_val: ParameterValue
}

impl SynthParam {
    pub fn new() -> Self {
        SynthParam{
            function: Function::Oscillator,
            function_id: FunctionId::Index(2),
            parameter: Parameter::Waveform,
            param_val: ParameterValue::Index(2)
        }
    }
}
