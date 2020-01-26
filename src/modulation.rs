use super::Float;
use super::{Parameter, MenuItem, ValueRange};
use super::{SynthParam, ParamId, FunctionId};
use super::Voice;
use super::voice::{NUM_OSCILLATORS, NUM_ENVELOPES, NUM_LFOS};
use super::synth::NUM_GLOBAL_LFOS;

use log::{info, trace, warn};
use serde::{Serialize, Deserialize};

/*
#[derive(Debug)]
pub enum ModValRange {
    IntRange(i64, i64),
    Float(Float, Float),
}

impl Default for ModValRange {
    fn default() -> Self { ModValRange::IntRange(0, 0) }
}
*/

/** Defines a source of modulation data and its value range. */
#[derive(Debug, Default)]
pub struct ModSource {
    pub function: Parameter,
    pub index_range: (usize, usize), // Min, Max
    pub val_range: ValueRange,
    pub is_global: bool,
}

/** Static list of available modulation data sources. */
static MOD_SOURCE: [ModSource; 8] = [
    ModSource{function: Parameter::GlobalLfo,  index_range: (1, NUM_GLOBAL_LFOS), val_range: ValueRange::Float(-1.0, 1.0, 0.1),  is_global: true},
    ModSource{function: Parameter::Aftertouch, index_range: (1, 1),               val_range: ValueRange::Float(0.0, 1.0, 0.1),   is_global: true},
    ModSource{function: Parameter::Pitchbend,  index_range: (1, 1),               val_range: ValueRange::Float(-1.0, 1.0, 0.01), is_global: true},
    ModSource{function: Parameter::ModWheel,   index_range: (1, 1),               val_range: ValueRange::Float(0.0, 127.0, 0.1), is_global: true},

    ModSource{function: Parameter::Envelope,   index_range: (1, NUM_ENVELOPES),   val_range: ValueRange::Float(0.0, 1.0, 0.01),  is_global: false},
    ModSource{function: Parameter::Lfo,        index_range: (1, NUM_LFOS),        val_range: ValueRange::Float(-1.0, 1.0, 0.01), is_global: false},
    ModSource{function: Parameter::Oscillator, index_range: (1, NUM_OSCILLATORS), val_range: ValueRange::Float(-1.0, 1.0, 0.01), is_global: false},
    ModSource{function: Parameter::Velocity,   index_range: (1, 1),               val_range: ValueRange::Float(0.0, 1.0, 0.1), is_global: false},
];

#[derive(Serialize, Deserialize, Copy, Clone, Debug, Default)]
pub struct ModData {
    pub source_func: Parameter,
    pub source_func_id: usize,
    pub target_func: Parameter,
    pub target_func_id: usize,
    pub target_param: Parameter,
    pub amount: Float,
    pub active: bool,
    pub is_global: bool,
    pub scale: Float,
}

impl ModData {
    pub fn new() -> ModData {
        let source_func = Parameter::Lfo;
        let source_func_id = 1;
        let target_func = Parameter::Oscillator;
        let target_func_id = 1;
        let target_param = Parameter::Level;
        let amount = 0.0;
        let active = false;
        let is_global = false;
        let scale = 0.0;
        ModData{source_func, source_func_id, target_func, target_func_id, target_param, amount, active, is_global, scale}
    }

    pub fn set_source(&mut self, func: &FunctionId) {
        self.source_func = func.function;
        self.source_func_id = func.function_id;
        self.update();
    }

    pub fn set_target(&mut self, param: &ParamId) {
        self.target_func = param.function;
        self.target_func_id = param.function_id;
        self.target_param = param.parameter;
        self.update();
    }

    pub fn set_amount(&mut self, amount: Float) {
        self.amount = amount;
        self.update();
    }

    pub fn update(&mut self) {
        // Modulation source
        let source = ModData::get_mod_source(self.source_func);
        let (source_min, source_max) = source.val_range.get_min_max();

        // Modulation target
        let dest_range = MenuItem::get_val_range(self.target_func, self.target_param);
        let (dest_min, dest_max) = dest_range.get_min_max();

        // Calculate scale factor
        // Scale is the factor applied to the mod source value to cover the
        // total target value range. Mod amount limits it to a smaller range.
        self.scale = ((dest_max - dest_min) / (source_max - source_min)) * self.amount;
        self.is_global = source.is_global;
        info!("Updated modulator {:?}", self);
    }

    pub fn get_source(&self) -> FunctionId {
        FunctionId{function: self.source_func, function_id: self.source_func_id, ..Default::default()}
    }

    pub fn get_target(&self) -> ParamId {
        ParamId{function: self.target_func, function_id: self.target_func_id, parameter: self.target_param}
    }

    fn get_mod_source(function: Parameter) -> &'static ModSource {
        for (i, s) in MOD_SOURCE.iter().enumerate() {
            if s.function == function {
                return &s;
            }
        }
        panic!();
    }
}

