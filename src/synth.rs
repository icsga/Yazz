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
    voice: [Voice; 16],
    keymap: [f32; 127],
    triggered: bool,
    num_voices_triggered: u32,
    voices_playing: u32, // Bitmap with currently playing voices
    trigger_seq: u64,
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
            Voice::new(sample_rate), Voice::new(sample_rate), Voice::new(sample_rate), Voice::new(sample_rate), Voice::new(sample_rate), Voice::new(sample_rate), Voice::new(sample_rate), Voice::new(sample_rate),
            Voice::new(sample_rate), Voice::new(sample_rate), Voice::new(sample_rate), Voice::new(sample_rate), Voice::new(sample_rate), Voice::new(sample_rate), Voice::new(sample_rate), Voice::new(sample_rate),
            /*
            Voice::new(sample_rate), Voice::new(sample_rate), Voice::new(sample_rate), Voice::new(sample_rate), Voice::new(sample_rate), Voice::new(sample_rate), Voice::new(sample_rate), Voice::new(sample_rate),
            Voice::new(sample_rate), Voice::new(sample_rate), Voice::new(sample_rate), Voice::new(sample_rate), Voice::new(sample_rate), Voice::new(sample_rate), Voice::new(sample_rate), Voice::new(sample_rate),
            */
        ];
        let mut keymap: [f32; 127] = [0.0; 127];
        Synth::calculate_keymap(&mut keymap, 440.0);
        let triggered = false;
        let num_voices_triggered = 0;
        let voices_playing = 0;
        let trigger_seq = 0;
        Synth{sample_rate, sound, voice, keymap, triggered, num_voices_triggered, voices_playing, trigger_seq, sender}
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
        if self.voices_playing > 0 {
            let sound = &self.sound.lock().unwrap();
            for i in 0..32 {
                if self.voices_playing & (1 << i) > 0 {
                    value += self.voice[i].get_sample(sample_clock, sound);
                }
            }
        }
        value
    }

    pub fn update(&mut self) {
        self.voices_playing = 0;
        for (i, v) in self.voice.iter_mut().enumerate() {
            if v.is_running() {
                self.voices_playing |= 1 << i;
            }
        }
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
                let voice_id = self.get_voice();
                self.voice[voice_id].set_key(msg.param);
                self.voice[voice_id].set_freq(freq);
                self.voice[voice_id].trigger(self.trigger_seq);
                self.num_voices_triggered += 1;
                self.trigger_seq += 1;
                self.voices_playing |= 1 << voice_id;
            }
            0x80 => {
                for (i, v) in self.voice.iter_mut().enumerate() {
                    if v.is_triggered() && v.key == msg.param {
                        self.num_voices_triggered -= 1;
                        v.release();
                        break;
                    }
                }
            }
            _ => ()
        }
    }

    /* Decide which voice gets to play the next note. */
    fn get_voice(&mut self) -> usize {
        let mut min_trigger_seq = std::u64::MAX;
        let mut min_id = 0;
        for (i, v) in self.voice.iter().enumerate() {
            if !v.is_running() {
                return i;
            }
            if v.trigger_seq < min_trigger_seq {
                min_trigger_seq = v.trigger_seq;
                min_id = i;
            }
        }
        min_id
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
