use super::DelayData;
use super::EnvelopeData;
use super::FilterData;
use super::Float;
use super::LfoData;
use super::ModData;
use super::WtOscData;
use super::{Parameter, ParameterValue, ParamId, SynthParam};

use log::{info, trace, warn};
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Copy, Clone, Debug)]
pub struct SoundData {
    pub osc: [WtOscData; 3],
    pub env: [EnvelopeData; 2],
    pub filter: [FilterData; 2],
    pub modul: [ModData; 16],
    pub lfo: [LfoData; 2],
    pub glfo: [LfoData; 2],
    pub delay: DelayData,
}

impl Default for SoundData {
    fn default() -> Self {
        SoundData::new()
    }
}

impl SoundData {
    pub fn new() -> SoundData {
        let osc = [
            WtOscData{..Default::default()},
            WtOscData{..Default::default()},
            WtOscData{..Default::default()},
        ];
        let env = [
            EnvelopeData{..Default::default()},
            EnvelopeData{..Default::default()},
        ];
        let filter = [
            FilterData{..Default::default()},
            FilterData{..Default::default()},
        ];
        let lfo = [
            LfoData{..Default::default()},
            LfoData{..Default::default()},
        ];
        let glfo = [
            LfoData{..Default::default()},
            LfoData{..Default::default()},
        ];
        let delay = DelayData{..Default::default()};
        let modul = [
            ModData::new(), ModData::new(), ModData::new(), ModData::new(),
            ModData::new(), ModData::new(), ModData::new(), ModData::new(),
            ModData::new(), ModData::new(), ModData::new(), ModData::new(),
            ModData::new(), ModData::new(), ModData::new(), ModData::new(),
        ];
        SoundData{osc, env, filter, lfo, glfo, delay, modul}
    }

    pub fn init(&mut self) {
        for o in self.osc.iter_mut() {
            o.init();
        }
        for e in self.env.iter_mut() {
            e.init();
        }
        for f in self.filter.iter_mut() {
            f.init();
        }
        for l in self.lfo.iter_mut() {
            l.init();
        }
        for g in self.glfo.iter_mut() {
            g.init();
        }
        self.osc[1].level = 0.0;
        self.osc[2].level = 0.0;
        self.delay.init();
    }

    pub fn get_osc_data<'a>(&'a self, id: usize) -> &'a WtOscData {
        &self.osc[id]
    }

    pub fn get_env_data<'a>(&'a self, id: usize) -> &'a EnvelopeData {
        &self.env[id]
    }

    pub fn set_parameter(&mut self, msg: &SynthParam) {
        let id = msg.function_id - 1;
        match msg.function {
            Parameter::Oscillator => {
                match msg.parameter {
                    Parameter::Level =>     { self.osc[id].level = if let ParameterValue::Float(x) = msg.value { x } else { panic!() } / 100.0; }
                    Parameter::WaveIndex => { self.osc[id].wave_index = if let ParameterValue::Float(x) = msg.value { x } else { panic!() }; }
                    Parameter::Frequency => { self.osc[id].set_halfsteps(if let ParameterValue::Int(x) = msg.value { x } else { panic!() }); }
                    Parameter::Finetune =>  { self.osc[id].set_cents(if let ParameterValue::Float(x) = msg.value { x / 100.0 } else { panic!() }); }
                    Parameter::Sync =>      { self.osc[id].sync = if let ParameterValue::Int(x) = msg.value { x } else { panic!() }; }
                    Parameter::KeyFollow => { self.osc[id].key_follow = if let ParameterValue::Int(x) = msg.value { x } else { panic!() }; }
                    Parameter::Voices =>    { self.osc[id].set_voice_num(if let ParameterValue::Int(x) = msg.value { x } else { panic!() }); }
                    Parameter::Spread =>    { self.osc[id].set_voice_spread(if let ParameterValue::Float(x) = msg.value { x } else { panic!() }); }
                    _ => {}
                }
            }
            Parameter::Filter => {
                match msg.parameter {
                    Parameter::Type =>      { self.filter[id].filter_type = if let ParameterValue::Choice(x) = msg.value { x } else { panic!() }; }
                    Parameter::Cutoff =>    { self.filter[id].cutoff = if let ParameterValue::Float(x) = msg.value { x } else { panic!() }; }
                    Parameter::Resonance => { self.filter[id].resonance = if let ParameterValue::Float(x) = msg.value { x } else { panic!() }; }
                    Parameter::Gain =>      { self.filter[id].gain = if let ParameterValue::Float(x) = msg.value { x } else { panic!() }; }
                    _ => {}
                }
            }
            Parameter::Amp => {}
            Parameter::Lfo => {
                match msg.parameter {
                    Parameter::Waveform =>  { self.lfo[id].select_wave(if let ParameterValue::Choice(x) = msg.value { x } else { panic!() }); }
                    Parameter::Frequency => { self.lfo[id].frequency = if let ParameterValue::Float(x) = msg.value { x } else { panic!() }; }
                    _ => {}
                }
            }
            Parameter::GlobalLfo => {
                match msg.parameter {
                    Parameter::Waveform =>  { self.lfo[id].select_wave(if let ParameterValue::Choice(x) = msg.value { x } else { panic!() }); }
                    Parameter::Frequency => { self.lfo[id].frequency = if let ParameterValue::Float(x) = msg.value { x } else { panic!() }; }
                    _ => {}
                }
            }
            Parameter::Envelope => {
                match msg.parameter {
                    Parameter::Attack =>  { self.env[id].attack = if let ParameterValue::Float(x) = msg.value { x } else { panic!() }; }
                    Parameter::Decay =>   { self.env[id].decay = if let ParameterValue::Float(x) = msg.value { x } else { panic!() }; }
                    Parameter::Sustain => { self.env[id].sustain = if let ParameterValue::Float(x) = msg.value { x } else { panic!() }; }
                    Parameter::Release => { self.env[id].release = if let ParameterValue::Float(x) = msg.value { x } else { panic!() }; }
                    Parameter::Factor =>  { self.env[id].factor = if let ParameterValue::Int(x) = msg.value { x as Float } else { panic!() }; }
                    _ => {}
                }
            }
            Parameter::Delay => {
                match msg.parameter {
                    Parameter::Time =>     { self.delay.time = if let ParameterValue::Float(x) = msg.value { x } else { panic!() }; }
                    Parameter::Level =>    { self.delay.level = if let ParameterValue::Float(x) = msg.value { x } else { panic!() }; }
                    Parameter::Feedback => { self.delay.feedback = if let ParameterValue::Float(x) = msg.value { x } else { panic!() }; }
                    Parameter::Tone =>     { self.delay.tone = if let ParameterValue::Float(x) = msg.value { x } else { panic!() }; }
                    _ => {}
                }
            }
            Parameter::Modulation => {
                match msg.parameter {
                    Parameter::Source => { if let ParameterValue::Function(x) = msg.value { self.modul[id].set_source(&x); } else { panic!() }; }
                    Parameter::Target => { if let ParameterValue::Param(x) = msg.value { self.modul[id].set_target(&x); } else { panic!() }; }
                    Parameter::Amount => { if let ParameterValue::Float(x) = msg.value { self.modul[id].set_amount(x) } else { panic!("{:?}", msg.value) }; }
                    Parameter::Active => { self.modul[id].active = if let ParameterValue::Int(x) = msg.value { x > 0 } else { panic!() }; }
                    _ => {}
                }
            }
            Parameter::System => {}
            _ => {}
        }
    }

    pub fn get_value(&self, param: &ParamId) -> ParameterValue {
        let id = param.function_id - 1;
        match param.function {
            Parameter::Oscillator => {
                match param.parameter {
                    Parameter::Level => ParameterValue::Float(self.osc[id].level * 100.0),
                    Parameter::WaveIndex => ParameterValue::Float(self.osc[id].wave_index),
                    Parameter::Frequency => ParameterValue::Int(self.osc[id].tune_halfsteps),
                    Parameter::Finetune => ParameterValue::Float(self.osc[id].tune_cents * 100.0),
                    Parameter::Sync => ParameterValue::Int(self.osc[id].sync),
                    Parameter::KeyFollow => ParameterValue::Int(self.osc[id].key_follow),
                    Parameter::Voices => ParameterValue::Int(self.osc[id].num_voices),
                    Parameter::Spread => ParameterValue::Float(self.osc[id].voice_spread),
                    _ => {panic!("Got ParamId {:?}", param);}
                }
            }
            Parameter::Filter => {
                match param.parameter {
                    Parameter::Type => ParameterValue::Choice(self.filter[id].filter_type),
                    Parameter::Cutoff => ParameterValue::Float(self.filter[id].cutoff),
                    Parameter::Resonance => ParameterValue::Float(self.filter[id].resonance),
                    Parameter::Gain => ParameterValue::Float(self.filter[id].gain),
                    _ => {panic!();}
                }
            }
            Parameter::Amp => {panic!();}
            Parameter::Lfo => {
                match param.parameter {
                    Parameter::Waveform =>  ParameterValue::Choice(self.lfo[id].get_waveform() as usize),
                    Parameter::Frequency => ParameterValue::Float(self.lfo[id].frequency),
                    _ => {panic!();}
                }
            }
            Parameter::GlobalLfo => {
                match param.parameter {
                    Parameter::Waveform =>  ParameterValue::Choice(self.lfo[id].get_waveform() as usize),
                    Parameter::Frequency => ParameterValue::Float(self.lfo[id].frequency),
                    _ => {panic!();}
                }
            }
            Parameter::Envelope => {
                match param.parameter {
                    Parameter::Attack => ParameterValue::Float(self.env[id].attack),
                    Parameter::Decay => ParameterValue::Float(self.env[id].decay),
                    Parameter::Sustain => ParameterValue::Float(self.env[id].sustain),
                    Parameter::Release => ParameterValue::Float(self.env[id].release),
                    Parameter::Factor => ParameterValue::Int(self.env[id].factor as i64),
                    _ => {panic!();}
                }
            }
            Parameter::Delay => {
                match param.parameter {
                    Parameter::Time => ParameterValue::Float(self.delay.time),
                    Parameter::Level => ParameterValue::Float(self.delay.level),
                    Parameter::Feedback => ParameterValue::Float(self.delay.feedback),
                    Parameter::Tone => ParameterValue::Float(self.delay.tone),
                    _ => {panic!();}
                }
            }
            Parameter::Modulation => {
                match param.parameter {
                    Parameter::Source => ParameterValue::Function(self.modul[id].get_source()),
                    Parameter::Target => ParameterValue::Param(self.modul[id].get_target()),
                    Parameter::Amount => ParameterValue::Float(self.modul[id].amount),
                    Parameter::Active => ParameterValue::Int(if self.modul[id].active { 1 } else { 0 }),
                    _ => {panic!();}
                }
            }
            Parameter::System => ParameterValue::NoValue,
            _ => {panic!();}
        }
    }

    /*
    pub fn insert_value(&self, msg: &mut SynthParam) {
        msg.value = self.get_value(msg);
    }

    fn insert_int(msg: &mut SynthParam, value: i64) {
        if let ParameterValue::Int(x) = &mut msg.value { *x = value; } else { panic!() };
    }

    fn insert_float(msg: &mut SynthParam, value: Float) {
        if let ParameterValue::Float(x) = &mut msg.value { *x = value; } else { panic!() };
    }

    fn insert_choice(msg: &mut SynthParam, value: usize) {
        if let ParameterValue::Choice(x) = &mut msg.value { *x = value; } else { panic!() };
    }
    */

    pub fn write(&self, filename: &str) {
        let serialized = serde_json::to_string(&self).unwrap();
        println!("serialized = {}", serialized);
        // TODO: Write to file
        let deserialized: SoundData = serde_json::from_str(&serialized).unwrap();
    }
}

