use super::{SynthMessage, UiMessage};
use super::midi_handler::{MessageType, MidiMessage};
use super::parameter::{Parameter, ParameterValue, SynthParam};
use super::voice::Voice;
use super::envelope::EnvelopeData;
use super::SampleGenerator;
use super::multi_oscillator::{MultiOscData, MultiOscillator};

use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::thread::spawn;

use crossbeam_channel::unbounded;
use crossbeam_channel::{Sender, Receiver};

pub enum Synth2UIMessage {
    Param(SynthParam),
    Control(u32),
    Log(String)
}

pub struct Synth {
    sample_rate: u32,
    sound: Arc<Mutex<SoundData>>,
    voice: [Voice; 1],
    keymap: [f32; 127],
    triggered: bool,
    voices_triggered: u32,
    sender: Sender<UiMessage>,
}

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
    }

    pub fn get_osc_data<'a>(&'a self, id: usize) -> &'a MultiOscData {
        &self.osc[id]
    }

    pub fn get_env_data<'a>(&'a self, id: usize) -> &'a EnvelopeData {
        &self.env[id]
    }
}

impl Synth {
    pub fn new(sample_rate: u32, sender: Sender<UiMessage>) -> Self {
        let mut sound = SoundData::new();
        sound.init();
        let sound = Arc::new(Mutex::new(sound));
        let voice = [
            Voice::new(sample_rate)
            /*
            Voice::new(sample_rate), Voice::new(sample_rate), Voice::new(sample_rate), Voice::new(sample_rate), Voice::new(sample_rate), Voice::new(sample_rate), Voice::new(sample_rate), Voice::new(sample_rate),
            Voice::new(sample_rate), Voice::new(sample_rate), Voice::new(sample_rate), Voice::new(sample_rate), Voice::new(sample_rate), Voice::new(sample_rate), Voice::new(sample_rate), Voice::new(sample_rate),
            Voice::new(sample_rate), Voice::new(sample_rate), Voice::new(sample_rate), Voice::new(sample_rate), Voice::new(sample_rate), Voice::new(sample_rate), Voice::new(sample_rate), Voice::new(sample_rate),
            Voice::new(sample_rate), Voice::new(sample_rate), Voice::new(sample_rate), Voice::new(sample_rate), Voice::new(sample_rate), Voice::new(sample_rate), Voice::new(sample_rate), Voice::new(sample_rate),
            */
        ];
        let mut keymap: [f32; 127] = [0.0; 127];
        Synth::calculate_keymap(&mut keymap, 440.0);
        let triggered = false;
        let voices_triggered = 0;
        Synth{sample_rate, sound, voice, keymap, triggered, voices_triggered, sender}
    }

    /* Starts a thread for receiving UI and MIDI messages. */
    pub fn run(synth: Arc<Mutex<Synth>>, synth_receiver: Receiver<SynthMessage>) -> std::thread::JoinHandle<()> {
        let handler = spawn(move || {
            loop {
                let msg = synth_receiver.recv().unwrap();
                let mut locked_synth = synth.lock().unwrap();
                match msg {
                    SynthMessage::Param(m) => locked_synth.handle_ui_message(m),
                    SynthMessage::ParamQuery(m) => locked_synth.handle_ui_query(m),
                    SynthMessage::Midi(m)  => locked_synth.handle_midi_message(m),
                    SynthMessage::WaveBuffer(m) => locked_synth.handle_wave_buffer(m),
                }
            }
        });
        handler
    }

    /* Called by the audio engine to get the next sample to be output. */
    pub fn get_sample(&mut self, sample_clock: i64) -> f32 {
        let mut value: f32 = 0.0;
        for v in self.voice.iter_mut() {
            value += v.get_sample(sample_clock, &self.sound.lock().unwrap());
        }
        value
    }

    /* Calculates the frequencies for the default keymap with equal temperament. */
    fn calculate_keymap(map: &mut[f32; 127], reference_pitch: f32) {
        for i in 0..127 {
           map[i] = (reference_pitch / 32.0) * (2.0f32.powf((i as f32 - 9.0) / 12.0));
        }
    }

    /* Handles a message received from the UI. */
    fn handle_ui_message(&mut self, msg: SynthParam) {
        let mut sound = self.sound.lock().unwrap();
        //let id = if let FunctionId::Int(x) = msg.function_id { x - 1 } else { panic!() } as usize;
        let id = msg.function_id - 1;
        match msg.function {
            Parameter::Oscillator => {
                match msg.parameter {
                    Parameter::Waveform => { sound.osc[id].select_wave(if let ParameterValue::Choice(x) = msg.value { x } else { panic!() }); }
                    Parameter::Level => { sound.osc[id].level = if let ParameterValue::Float(x) = msg.value { x } else { panic!() } / 100.0; }
                    Parameter::Frequency => { sound.osc[id].set_freq_offset(if let ParameterValue::Int(x) = msg.value { x } else { panic!() }); }
                    Parameter::Blend => { sound.osc[id].set_ratio(if let ParameterValue::Float(x) = msg.value { x } else { panic!() }); }
                    Parameter::Phase => { sound.osc[id].phase = if let ParameterValue::Float(x) = msg.value { x } else { panic!() }; }
                    Parameter::Sync => { sound.osc[id].sync = if let ParameterValue::Int(x) = msg.value { x } else { panic!() }; }
                    _ => {}
                }
            }
            Parameter::Filter => {}
            Parameter::Amp => {}
            Parameter::Lfo => {}
            Parameter::Envelope => {
                match msg.parameter {
                    Parameter::Attack => { sound.env[id].attack = if let ParameterValue::Float(x) = msg.value { x } else { panic!() }; }
                    Parameter::Decay => { sound.env[id].decay = if let ParameterValue::Float(x) = msg.value { x } else { panic!() }; }
                    Parameter::Sustain => { sound.env[id].sustain = if let ParameterValue::Float(x) = msg.value { x } else { panic!() } / 100.0; }
                    Parameter::Release => { sound.env[id].release = if let ParameterValue::Float(x) = msg.value { x } else { panic!() }; }
                    _ => {}
                }
            }
            Parameter::Mod => {}
            Parameter::System => {}
            _ => {}
        }
    }

    fn handle_ui_query(&mut self, mut msg: SynthParam) {
        let sound = self.sound.lock().unwrap();
        //let id = if let FunctionId::Int(x) = msg.function_id { x - 1 } else { panic!() } as usize;
        let id = msg.function_id - 1;
        match msg.function {
            Parameter::Oscillator => {
                match msg.parameter {
                    Parameter::Waveform => {
                        if let ParameterValue::Choice(x) = &mut msg.value { *x = sound.osc[id].get_waveform() as usize; } else { panic!() };
                    }
                    Parameter::Level => {
                        if let ParameterValue::Float(x) = &mut msg.value { *x = sound.osc[id].level * 100.0; } else { panic!() };
                    }
                    Parameter::Frequency => {
                        if let ParameterValue::Int(x) = &mut msg.value { *x = sound.osc[id].tune_halfsteps; } else { panic!() };
                    }
                    Parameter::Phase => {
                        if let ParameterValue::Float(x) = &mut msg.value { *x = sound.osc[id].phase; } else { panic!() };
                    }
                    Parameter::Sync => {
                        if let ParameterValue::Int(x) = &mut msg.value { *x = sound.osc[id].sync; } else { panic!() };
                    }
                    _ => {}
                }
            }
            Parameter::Filter => {}
            Parameter::Amp => {
            }
            Parameter::Lfo => {}
            Parameter::Envelope => {
                if let ParameterValue::Float(x) = &mut msg.value {
                    *x = match msg.parameter {
                        Parameter::Attack => sound.env[id].attack,
                        Parameter::Decay => sound.env[id].decay,
                        Parameter::Sustain => sound.env[id].sustain,
                        Parameter::Release => sound.env[id].release,
                        _ => panic!()
                    };
                } else { panic!() };
            }
            Parameter::Mod => {}
            Parameter::System => {}
            _ => {}
        }
        self.sender.send(UiMessage::Param(msg)).unwrap();
    }

    /* Handles a received MIDI message. */
    fn handle_midi_message(&mut self, msg: MidiMessage) {
        let channel = msg.mtype & 0x0F;
        let mtype: u8 = msg.mtype & 0xF0;
        match mtype {
            0x90 => {
                let freq = self.keymap[msg.param as usize];
                self.voice[0].set_freq(freq);
                self.voice[0].trigger();
                self.voices_triggered += 1;
            }
            0x80 => {
                self.voices_triggered -= 1;
                if self.voices_triggered == 0 {
                    self.voice[0].release();
                }
            }
            _ => ()
        }
    }

    fn handle_wave_buffer(&mut self, mut buffer: Vec<f32>) {
        let len = buffer.capacity();
        let mut osc = MultiOscillator::new(44100, 0);
        for i in 0..buffer.capacity() {
            let (sample, complete) = osc.get_sample(440.0, i as i64, &self.sound.lock().unwrap(), false);
            buffer[i] = sample;
        }
        self.sender.send(UiMessage::WaveBuffer(buffer)).unwrap();
    }
}
