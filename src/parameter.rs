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
    Factor,

    // Mod
    Source,
    Target,

    // Waveforms
    Sine,
    Triangle,
    Saw,
    Square,
    Noise,
}

#[derive(Clone, Copy, Debug)]
pub enum ParameterValue {
    Int(i64),
    Float(f32),
    Choice(usize),
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
