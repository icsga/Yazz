use super::envelope::EnvelopeData;
use super::multi_oscillator::MultiOscData;
use super::parameter::{Parameter, ParameterValue, SynthParam};

#[derive(Default)]
pub struct SoundData {
    pub osc: [MultiOscData; 3],
    pub env: [EnvelopeData; 2],
}

impl SoundData {
    pub fn new() -> SoundData {
        let osc = [
            MultiOscData{..Default::default()},
            MultiOscData{..Default::default()},
            MultiOscData{..Default::default()},
        ];
        let env = [
            EnvelopeData{..Default::default()},
            EnvelopeData{..Default::default()},
        ];
        SoundData{osc, env}
    }

    pub fn init(&mut self) {
        for o in self.osc.iter_mut() {
            o.init();
        }
        for e in self.env.iter_mut() {
            e.init();
        }
        self.osc[1].level = 0.0;
        self.osc[2].level = 0.0;
    }

    pub fn get_osc_data<'a>(&'a self, id: usize) -> &'a MultiOscData {
        &self.osc[id]
    }

    pub fn get_env_data<'a>(&'a self, id: usize) -> &'a EnvelopeData {
        &self.env[id]
    }

    pub fn set_parameter(&mut self, msg: SynthParam) {
        let id = msg.function_id - 1;
        match msg.function {
            Parameter::Oscillator => {
                match msg.parameter {
                    Parameter::Waveform => { self.osc[id].select_wave(if let ParameterValue::Choice(x) = msg.value { x } else { panic!() }); }
                    Parameter::Level => { self.osc[id].level = if let ParameterValue::Float(x) = msg.value { x } else { panic!() } / 100.0; }
                    Parameter::Frequency => { self.osc[id].set_freq_offset(if let ParameterValue::Int(x) = msg.value { x } else { panic!() }); }
                    Parameter::Blend => { self.osc[id].set_ratio(if let ParameterValue::Float(x) = msg.value { x } else { panic!() }); }
                    Parameter::Phase => { self.osc[id].phase = if let ParameterValue::Float(x) = msg.value { x } else { panic!() }; }
                    Parameter::Sync => { self.osc[id].sync = if let ParameterValue::Int(x) = msg.value { x } else { panic!() }; }
                    Parameter::KeyFollow => { self.osc[id].key_follow = if let ParameterValue::Int(x) = msg.value { x } else { panic!() }; }
                    _ => {}
                }
            }
            Parameter::Filter => {}
            Parameter::Amp => {}
            Parameter::Lfo => {}
            Parameter::Envelope => {
                match msg.parameter {
                    Parameter::Attack => { self.env[id].attack = if let ParameterValue::Float(x) = msg.value { x } else { panic!() }; }
                    Parameter::Decay => { self.env[id].decay = if let ParameterValue::Float(x) = msg.value { x } else { panic!() }; }
                    Parameter::Sustain => { self.env[id].sustain = if let ParameterValue::Float(x) = msg.value { x } else { panic!() } / 100.0; }
                    Parameter::Release => { self.env[id].release = if let ParameterValue::Float(x) = msg.value { x } else { panic!() }; }
                    _ => {}
                }
            }
            Parameter::Mod => {}
            Parameter::System => {}
            _ => {}
        }
    }

    pub fn insert_value(&self, msg: &mut SynthParam) {
        let id = msg.function_id - 1;
        match msg.function {
            Parameter::Oscillator => {
                match msg.parameter {
                    Parameter::Waveform => SoundData::insert_choice(msg, self.osc[id].get_waveform() as usize),
                    Parameter::Level => SoundData::insert_float(msg, self.osc[id].level * 100.0),
                    Parameter::Frequency => SoundData::insert_int(msg, self.osc[id].tune_halfsteps),
                    Parameter::Phase => SoundData::insert_float(msg, self.osc[id].phase),
                    Parameter::Sync => SoundData::insert_int(msg, self.osc[id].sync),
                    Parameter::KeyFollow => SoundData::insert_int(msg, self.osc[id].key_follow),
                    _ => {}
                }
            }
            Parameter::Filter => {}
            Parameter::Amp => {
            }
            Parameter::Lfo => {}
            Parameter::Envelope => {
                match msg.parameter {
                    Parameter::Attack => SoundData::insert_float(msg, self.env[id].attack),
                    Parameter::Decay => SoundData::insert_float(msg, self.env[id].decay),
                    Parameter::Sustain => SoundData::insert_float(msg, self.env[id].sustain),
                    Parameter::Release => SoundData::insert_float(msg, self.env[id].release),
                    _ => panic!()
                }
            }
            Parameter::Mod => {}
            Parameter::System => {}
            _ => {}
        }
    }

    fn insert_int(msg: &mut SynthParam, value: i64) {
        if let ParameterValue::Int(x) = &mut msg.value { *x = value; } else { panic!() };
    }

    fn insert_float(msg: &mut SynthParam, value: f32) {
        if let ParameterValue::Float(x) = &mut msg.value { *x = value; } else { panic!() };
    }

    fn insert_choice(msg: &mut SynthParam, value: usize) {
        if let ParameterValue::Choice(x) = &mut msg.value { *x = value; } else { panic!() };
    }
}

