use super::Float;
use super::Parameter;
use super::{SynthParam, ParamId, FunctionId};
use super::Voice;
use super::voice::{NUM_OSCILLATORS, NUM_ENVELOPES, NUM_LFOS};
use super::synth::NUM_GLOBAL_LFOS;

use log::{info, trace, warn};
use serde::{Serialize, Deserialize};

#[derive(Debug)]
pub enum ModValRange {
    IntRange(i64, i64),
    FloatRange(Float, Float),
}

impl Default for ModValRange {
    fn default() -> Self { ModValRange::IntRange(0, 0) }
}

/** Defines a source of modulation data and its value range. */
#[derive(Debug, Default)]
pub struct ModSource {
    pub function: Parameter,
    pub index_range: (usize, usize), // Min, Max
    pub val_range: ModValRange,
    pub is_global: bool,
}

/** Static list of available modulation data sources. */
static MOD_SOURCE: [ModSource; 5] = [
    ModSource{function: Parameter::Envelope, index_range: (1, NUM_ENVELOPES), val_range: ModValRange::FloatRange(0.0, 1.0), is_global: false},
    ModSource{function: Parameter::Lfo, index_range: (1, NUM_LFOS), val_range: ModValRange::FloatRange(-1.0, 1.0), is_global: false},
    ModSource{function: Parameter::Oscillator, index_range: (1, NUM_OSCILLATORS), val_range: ModValRange::FloatRange(-1.0, 1.0), is_global: false},

    ModSource{function: Parameter::KeyAttack, index_range: (1, 1), val_range: ModValRange::IntRange(0, 127), is_global: true},
    ModSource{function: Parameter::GlobalLfo, index_range: (1, NUM_GLOBAL_LFOS), val_range: ModValRange::FloatRange(-1.0, 1.0), is_global: true},
];

/** Defines a target for modulation data with it's allowed value range. */
#[derive(Debug, Default)]
pub struct ModDest {
    pub function: Parameter,
    pub parameter: Parameter,
    pub val_min: Float,
    pub val_max: Float,
}

static MOD_DEST: [ModDest; 6] = [
    ModDest{function: Parameter::Oscillator, parameter: Parameter::Level,      val_min: 0.0,   val_max: 100.0},
    ModDest{function: Parameter::Oscillator, parameter: Parameter::Finetune,   val_min: 0.0,   val_max: 1200.0},
    ModDest{function: Parameter::Oscillator, parameter: Parameter::Blend,      val_min: 0.0,   val_max: 5.0},
    ModDest{function: Parameter::Oscillator, parameter: Parameter::Phase,      val_min: 0.0,   val_max: 1.0},
    ModDest{function: Parameter::Oscillator, parameter: Parameter::Voices,     val_min: 1.0,   val_max: 7.0},
    ModDest{function: Parameter::Delay,      parameter: Parameter::Time,       val_min: 0.0,   val_max: 1.0},
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
    pub offset: Float,
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
        let offset = 0.0;
        ModData{source_func, source_func_id, target_func, target_func_id, target_param, amount, active, is_global, scale, offset}
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
        // Lookup input
        let source = ModData::get_mod_source(self.source_func);

        // Lookup output
        let dest = ModData::get_mod_dest(self.target_func, self.target_param);

        let scale: Float;
        let offset: Float;

        // Calculate scale factor and offset
        match source.val_range {
            ModValRange::IntRange(min, max) => {
                scale = (dest.val_max - dest.val_min) / (max - min) as Float;
                //offset = (dest.val_min - min as Float) * scale;
                offset = dest.val_min - (min as Float * scale);
                info!("min={}, max={}, dest_min={}, dest_max={}, scale={}, offset={}",
                    min, max, dest.val_min, dest.val_max, scale, offset);
            }
            ModValRange::FloatRange(min, max) => {
                scale = (dest.val_max - dest.val_min) / (max - min);
                offset = dest.val_min - (min * scale);
                info!("min={}, max={}, dest_min={}, dest_max={}, scale={}, offset={}",
                    min, max, dest.val_min, dest.val_max, scale, offset);
            }
        }
        self.scale =  scale * self.amount;
        self.offset =  offset;
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

    fn get_mod_dest(function: Parameter, parameter: Parameter) -> &'static ModDest {
        for (i, d) in MOD_DEST.iter().enumerate() {
            if d.function == function && d.parameter == parameter {
                return &d;
            }
        }
        panic!("{:?}, {:?}", function, parameter);
    }
}

/* Modulator
 *
 * 1. Get the input source
 * 2. Get current value from input source
 * 3. Scale input value to match destination format
 * 4. Update destination parameter:
 *    - Direct (runtime parameter, e.g. frequency): Apply directly
 *    - Config-based: Change config according to source:
 *      * Global config (GLFOs, Aftertouch, MIDI clock, ...)
 *      * Voice config (LFOs, key value, velocity, ...)
 *
 * Needs multiple passes:
 * - Once for global config and direct parameters
 * - Once per voice for voice-specifig parameters
 *
 */

#[derive(Debug, Default)]
pub struct Modulator {
}

impl Modulator {
}
