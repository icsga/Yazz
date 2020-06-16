use super::DelayData;
use super::EnvelopeData;
use super::FilterData;
use super::Float;
use super::LfoData;
use super::ModData;
use super::{OscData, OscType, OscRouting};
use super::synth::*;
use super::voice::*;
use super::{Parameter, ParameterValue, ParamId, SynthParam};

use serde::{Serialize, Deserialize};

// TODO: These are used in two places. Find a better place for them.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub enum SyncValue {
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

impl SyncValue {
    pub fn from_int(param: usize) -> SyncValue {
        match param {
            0 => SyncValue::Off,
            1 => SyncValue::Whole,
            2 => SyncValue::DottedHalf,
            3 => SyncValue::Half,
            4 => SyncValue::DottedQuarter,
            5 => SyncValue::Quarter,
            6 => SyncValue::DottedEigth,
            7 => SyncValue::Eigth,
            8 => SyncValue::Sixteenth,
            _ => panic!(),
        }
    }
}

impl Default for SyncValue {
    fn default() -> Self { SyncValue::Off }
}

/** Sound data
 *
 * \todo Separate voice and global sound data, pack voice data in it's own
 *       struct for faster copying on sample calculation.
 */
#[derive(Serialize, Deserialize, Copy, Clone, Debug)]
pub struct SoundData {
    pub osc: [OscData; NUM_OSCILLATORS],
    pub env: [EnvelopeData; NUM_ENVELOPES],
    pub filter: [FilterData; NUM_FILTERS],
    pub lfo: [LfoData; NUM_LFOS],
    pub glfo: [LfoData; NUM_GLOBAL_LFOS],
    pub modul: [ModData; NUM_MODULATORS],
    pub delay: DelayData,
    pub patch: PatchData,
}

impl Default for SoundData {
    fn default() -> Self {
        SoundData::new()
    }
}

impl SoundData {
    pub fn new() -> SoundData {
        let osc = [
            OscData{..Default::default()},
            OscData{..Default::default()},
            OscData{..Default::default()},
        ];
        let env = [
            EnvelopeData{..Default::default()},
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
        let modul = [
            ModData::new(), ModData::new(), ModData::new(), ModData::new(),
            ModData::new(), ModData::new(), ModData::new(), ModData::new(),
            ModData::new(), ModData::new(), ModData::new(), ModData::new(),
            ModData::new(), ModData::new(), ModData::new(), ModData::new(),
        ];
        let delay = DelayData{..Default::default()};
        let patch = PatchData{..Default::default()};
        SoundData{osc, env, filter, lfo, glfo, modul, delay, patch}
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
        self.patch.init();
    }

    pub fn get_osc_data<'a>(&'a self, id: usize) -> &'a OscData {
        &self.osc[id]
    }

    pub fn get_env_data<'a>(&'a self, id: usize) -> &'a EnvelopeData {
        &self.env[id]
    }

    pub fn set_parameter(&mut self, msg: &SynthParam) {
        let id = msg.function_id - 1;
        match msg.function {
            Parameter::Oscillator => {
                let osc = &mut self.osc[id];
                match msg.parameter {
                    Parameter::Level =>     { osc.level = if let ParameterValue::Float(x) = msg.value { x } else { panic!() } / 100.0; }
                    Parameter::Tune =>      { osc.set_halfsteps(if let ParameterValue::Int(x) = msg.value { x } else { panic!() }); }
                    Parameter::Finetune =>  { osc.set_cents(if let ParameterValue::Float(x) = msg.value { x / 100.0 } else { panic!() }); }
                    Parameter::Sync =>      { osc.sync = if let ParameterValue::Int(x) = msg.value { x } else { panic!() }; }
                    Parameter::KeyFollow => { osc.key_follow = if let ParameterValue::Int(x) = msg.value { x } else { panic!() }; }
                    Parameter::Routing =>   { osc.routing = if let ParameterValue::Choice(x) = msg.value { OscRouting::from_int(x) } else { panic!() }; }
                    Parameter::Type =>      { osc.osc_type = if let ParameterValue::Choice(x) = msg.value { OscType::from_int(x) } else { panic!() }; }
                    // WtOsc
                    Parameter::Wavetable => { osc.wt_osc_data.wavetable = if let ParameterValue::Dynamic(_, x) = msg.value { x } else { panic!() }; }
                    Parameter::WaveIndex => { osc.wt_osc_data.wave_index = if let ParameterValue::Float(x) = msg.value { x } else { panic!() }; }
                    Parameter::Voices =>    { osc.wt_osc_data.set_voice_num(if let ParameterValue::Int(x) = msg.value { x } else { panic!() }); }
                    Parameter::Spread =>    { osc.wt_osc_data.set_voice_spread(if let ParameterValue::Float(x) = msg.value { x } else { panic!() }); }
                    _ => {}
                }
            }
            Parameter::Filter => {
                match msg.parameter {
                    Parameter::Type =>      { self.filter[id].filter_type = if let ParameterValue::Choice(x) = msg.value { x } else { panic!() }; }
                    Parameter::Cutoff =>    { self.filter[id].cutoff = if let ParameterValue::Float(x) = msg.value { x } else { panic!() }; }
                    Parameter::Resonance => { self.filter[id].resonance = if let ParameterValue::Float(x) = msg.value { x } else { panic!() }; }
                    Parameter::Gain =>      { self.filter[id].gain = if let ParameterValue::Float(x) = msg.value { x } else { panic!() }; }
                    Parameter::Aux =>       { self.filter[id].aux = if let ParameterValue::Float(x) = msg.value { x } else { panic!() }; }
                    Parameter::KeyFollow => { self.filter[id].key_follow = if let ParameterValue::Int(x) = msg.value { x } else { panic!() }; }
                    _ => {}
                }
            }
            Parameter::Amp => {}
            Parameter::Lfo => {
                let mut lfo = &mut self.lfo[id];
                match msg.parameter {
                    Parameter::Waveform =>  { lfo.select_wave(if let ParameterValue::Choice(x) = msg.value { x } else { panic!() }); }
                    Parameter::Frequency => { lfo.frequency = if let ParameterValue::Float(x) = msg.value { x } else { panic!() }; }
                    Parameter::Sync =>      { lfo.sync = if let ParameterValue::Choice(x) = msg.value { SyncValue::from_int(x) } else { panic!() }; }
                    Parameter::Phase =>     { lfo.phase = if let ParameterValue::Float(x) = msg.value { x } else { panic!() }; }
                    Parameter::Amount =>    { lfo.amount = if let ParameterValue::Float(x) = msg.value { x } else { panic!() }; }
                    _ => {}
                }
            }
            Parameter::GlobalLfo => {
                let mut glfo = &mut self.glfo[id];
                match msg.parameter {
                    Parameter::Waveform =>  { glfo.select_wave(if let ParameterValue::Choice(x) = msg.value { x } else { panic!() }); }
                    Parameter::Frequency => { glfo.frequency = if let ParameterValue::Float(x) = msg.value { x } else { panic!() }; }
                    Parameter::Sync =>      { glfo.sync = if let ParameterValue::Choice(x) = msg.value { SyncValue::from_int(x) } else { panic!() }; }
                    Parameter::Phase =>     { glfo.phase = if let ParameterValue::Float(x) = msg.value { x } else { panic!() }; }
                    Parameter::Amount =>    { glfo.amount = if let ParameterValue::Float(x) = msg.value { x } else { panic!() }; }
                    _ => {}
                }
            }
            Parameter::Envelope => {
                match msg.parameter {
                    Parameter::Attack =>      { self.env[id].attack = if let ParameterValue::Float(x) = msg.value { x } else { panic!() }; }
                    Parameter::Decay =>       { self.env[id].decay = if let ParameterValue::Float(x) = msg.value { x } else { panic!() }; }
                    Parameter::Sustain =>     { self.env[id].sustain = if let ParameterValue::Float(x) = msg.value { x } else { panic!() }; }
                    Parameter::Release =>     { self.env[id].release = if let ParameterValue::Float(x) = msg.value { x } else { panic!() }; }
                    Parameter::Factor =>      { self.env[id].factor = if let ParameterValue::Int(x) = msg.value { x as Float } else { panic!() }; }
                    Parameter::Delay =>       { self.env[id].delay = if let ParameterValue::Float(x) = msg.value { x } else { panic!() }; }
                    Parameter::Loop =>        { self.env[id].looping = if let ParameterValue::Int(x) = msg.value { x > 0 } else { panic!() }; }
                    Parameter::ResetToZero => { self.env[id].reset_to_zero = if let ParameterValue::Int(x) = msg.value { x > 0 } else { panic!() }; }
                    _ => {}
                }
            }
            Parameter::Delay => {
                match msg.parameter {
                    Parameter::Time =>     { self.delay.time = if let ParameterValue::Float(x) = msg.value { x } else { panic!() }; }
                    Parameter::Sync =>     { self.delay.sync = if let ParameterValue::Choice(x) = msg.value { SyncValue::from_int(x) } else { panic!() }; }
                    Parameter::Level =>    { self.delay.level = if let ParameterValue::Float(x) = msg.value { x } else { panic!() }; }
                    Parameter::Feedback => { self.delay.feedback = if let ParameterValue::Float(x) = msg.value { x } else { panic!() }; }
                    Parameter::Tone =>     { self.delay.tone = if let ParameterValue::Float(x) = msg.value { x } else { panic!() }; }
                    Parameter::Type =>     { self.delay.delay_type = if let ParameterValue::Choice(x) = msg.value { x } else { panic!() }; }
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
            Parameter::Patch => {
                match msg.parameter {
                    Parameter::Level => { self.patch.level = if let ParameterValue::Float(x) = msg.value { x } else { panic!() } / 100.0; }
                    Parameter::Drive => { self.patch.drive = if let ParameterValue::Float(x) = msg.value { x } else { panic!() }; }
                    Parameter::Pitchbend => { self.patch.pitchbend = if let ParameterValue::Int(x) = msg.value { x as Float } else { panic!() }; }
                    Parameter::VelSens => { self.patch.vel_sens = if let ParameterValue::Float(x) = msg.value { x } else { panic!() }; }
                    Parameter::EnvDepth => { self.patch.env_depth = if let ParameterValue::Float(x) = msg.value { x } else { panic!() }; }
                    Parameter::PlayMode => { self.patch.play_mode = if let ParameterValue::Choice(x) = msg.value { PlayMode::from_int(x) } else { panic!() }; }
                    Parameter::FilterRouting => { self.patch.filter_routing = if let ParameterValue::Choice(x) = msg.value { FilterRouting::from_int(x) } else { panic!() }; }
                    Parameter::Bpm => { self.patch.bpm = if let ParameterValue::Float(x) = msg.value { x } else { panic!() }; }
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
                let osc = &self.osc[id];
                match param.parameter {
                    Parameter::Level => ParameterValue::Float(osc.level * 100.0),
                    Parameter::Tune => ParameterValue::Int(osc.tune_halfsteps),
                    Parameter::Finetune => ParameterValue::Float(osc.tune_cents * 100.0),
                    Parameter::Sync => ParameterValue::Int(osc.sync),
                    Parameter::KeyFollow => ParameterValue::Int(osc.key_follow),
                    Parameter::Routing => ParameterValue::Choice(osc.routing.to_int()),
                    Parameter::Type => ParameterValue::Choice(osc.osc_type.to_int()),
                    // WtOsc
                    Parameter::Wavetable => ParameterValue::Dynamic(Parameter::Wavetable, osc.wt_osc_data.wavetable),
                    Parameter::WaveIndex => ParameterValue::Float(osc.wt_osc_data.wave_index),
                    Parameter::Voices => ParameterValue::Int(osc.wt_osc_data.num_voices),
                    Parameter::Spread => ParameterValue::Float(osc.wt_osc_data.voice_spread),
                    _ => {panic!("Got ParamId {:?}", param);}
                }
            }
            Parameter::Filter => {
                let filter = &self.filter[id];
                match param.parameter {
                    Parameter::Type => ParameterValue::Choice(filter.filter_type),
                    Parameter::Cutoff => ParameterValue::Float(filter.cutoff),
                    Parameter::Resonance => ParameterValue::Float(filter.resonance),
                    Parameter::Gain => ParameterValue::Float(filter.gain),
                    Parameter::Aux => ParameterValue::Float(filter.aux),
                    Parameter::KeyFollow => ParameterValue::Int(filter.key_follow),
                    _ => {panic!();}
                }
            }
            Parameter::Amp => {panic!();}
            Parameter::Lfo => {
                let lfo = &self.lfo[id];
                match param.parameter {
                    Parameter::Waveform =>  ParameterValue::Choice(lfo.get_waveform() as usize),
                    Parameter::Frequency => ParameterValue::Float(lfo.frequency),
                    Parameter::Sync => ParameterValue::Choice(lfo.sync as usize),
                    Parameter::Phase => ParameterValue::Float(lfo.phase),
                    Parameter::Amount => ParameterValue::Float(lfo.amount),
                    _ => {panic!();}
                }
            }
            Parameter::GlobalLfo => {
                let glfo = &self.glfo[id];
                match param.parameter {
                    Parameter::Waveform =>  ParameterValue::Choice(glfo.get_waveform() as usize),
                    Parameter::Frequency => ParameterValue::Float(glfo.frequency),
                    Parameter::Sync => ParameterValue::Choice(glfo.sync as usize),
                    Parameter::Phase => ParameterValue::Float(glfo.phase),
                    Parameter::Amount => ParameterValue::Float(glfo.amount),
                    _ => {panic!();}
                }
            }
            Parameter::Envelope => {
                let env = &self.env[id];
                match param.parameter {
                    Parameter::Attack => ParameterValue::Float(env.attack),
                    Parameter::Decay => ParameterValue::Float(env.decay),
                    Parameter::Sustain => ParameterValue::Float(env.sustain),
                    Parameter::Release => ParameterValue::Float(env.release),
                    Parameter::Factor => ParameterValue::Int(env.factor as i64),
                    Parameter::Delay => ParameterValue::Float(env.delay),
                    Parameter::Loop => ParameterValue::Int(if env.looping { 1 } else { 0 }),
                    Parameter::ResetToZero => ParameterValue::Int(if env.reset_to_zero { 1 } else { 0 }),
                    _ => {panic!();}
                }
            }
            Parameter::Delay => {
                match param.parameter {
                    Parameter::Time => ParameterValue::Float(self.delay.time),
                    Parameter::Sync => ParameterValue::Choice(self.delay.sync as usize),
                    Parameter::Level => ParameterValue::Float(self.delay.level),
                    Parameter::Feedback => ParameterValue::Float(self.delay.feedback),
                    Parameter::Tone => ParameterValue::Float(self.delay.tone),
                    Parameter::Type => ParameterValue::Choice(self.delay.delay_type),
                    _ => {panic!();}
                }
            }
            Parameter::Modulation => {
                let modul = &self.modul[id];
                match param.parameter {
                    Parameter::Source => ParameterValue::Function(modul.get_source()),
                    Parameter::Target => ParameterValue::Param(modul.get_target()),
                    Parameter::Amount => ParameterValue::Float(modul.amount),
                    Parameter::Active => ParameterValue::Int(if modul.active { 1 } else { 0 }),
                    _ => {panic!();}
                }
            }
            Parameter::Patch => {
                match param.parameter {
                    Parameter::Level => ParameterValue::Float(self.patch.level * 100.0),
                    Parameter::Drive => ParameterValue::Float(self.patch.drive),
                    Parameter::Pitchbend => ParameterValue::Int(self.patch.pitchbend as i64),
                    Parameter::VelSens => ParameterValue::Float(self.patch.vel_sens),
                    Parameter::EnvDepth => ParameterValue::Float(self.patch.env_depth),
                    Parameter::PlayMode => ParameterValue::Choice(self.patch.play_mode as usize),
                    Parameter::FilterRouting => ParameterValue::Choice(self.patch.filter_routing as usize),
                    Parameter::Bpm => ParameterValue::Float(self.patch.bpm),
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

    pub fn write(&self, _filename: &str) {
        let serialized = serde_json::to_string(&self).unwrap();
        println!("serialized = {}", serialized);
        // TODO: Write to file
        //let deserialized: SoundData = serde_json::from_str(&serialized).unwrap();
    }
}

