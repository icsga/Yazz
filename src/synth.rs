use super::midi_handler::{MessageType, MidiMessage};
use super::parameter::{FunctionId, Parameter, ParameterValue, SynthParam};
use super::voice::Voice;

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
    voice: Voice,
    keymap: [f32; 127],
    triggered: bool,
    voices_triggered: u32,
    sender: Sender<Synth2UIMessage>,
}

/* Data for a single sound patch. */
struct Patch {
    voice_params: Rc<VoiceParams>,
}

struct VoiceParams {
    osc_params: [OscillatorParams; 3],
}

struct OscillatorParams {
    waveform: Parameter,
}

impl Synth {
    pub fn new(sample_rate: u32, sender: Sender<Synth2UIMessage>) -> Self {
        //let voices = [Voice::new(sample_rate); 16];
        let voice = Voice::new(sample_rate);
        let mut keymap: [f32; 127] = [0.0; 127];
        Synth::calculate_keymap(&mut keymap, 440.0);
        let triggered = false;
        let voices_triggered = 0;
        Synth{sample_rate, voice, keymap, triggered, voices_triggered, sender}
    }

    /* Starts a thread for receiving UI and MIDI messages. */
    pub fn run(synth: Arc<Mutex<Synth>>, t2s_receiver: Receiver<SynthParam>, m2s_receiver: Receiver<MidiMessage>) -> std::thread::JoinHandle<()> {
        let handler = spawn(move || {
            loop {
                select! {
                    recv(t2s_receiver) -> msg => {
                        let mut locked_synth = synth.lock().unwrap();
                        locked_synth.handle_ui_message(msg.unwrap());
                    },
                    recv(m2s_receiver) -> msg => {
                        let mut locked_synth = synth.lock().unwrap();
                        locked_synth.handle_midi_message(msg.unwrap());
                    },
                }
            }
        });
        handler
    }

    /* Called by the audio engine to get the next sample to be output. */
    pub fn get_sample(&mut self, sample_clock: u64) -> f32 {
        self.voice.get_sample(sample_clock)
    }

    /* Calculates the frequencies for the default keymap with equal temperament. */
    fn calculate_keymap(map: &mut[f32; 127], reference_pitch: f32) {
        for i in 0..127 {
           map[i] = (reference_pitch / 32.0) * (2.0f32.powf((i as f32 - 9.0) / 12.0));
        }
    }

    /* Handles a message received from the UI. */
    fn handle_ui_message(&mut self, msg: SynthParam) {
        match msg.function {
            Parameter::Oscillator => {
                match msg.parameter {
                    Parameter::Waveform => {
                        let value = if let ParameterValue::Choice(x) = msg.value { x } else { panic!() };
                        self.voice.set_wave_ratio(value);
                    }
                    _ => {}
                }
            }
            Parameter::Filter => {}
            Parameter::Amp => {}
            Parameter::Lfo => {}
            Parameter::Envelope => {}
            Parameter::Mod => {}
            Parameter::System => {}
            _ => {}
        }
    }

    /* Handles a received MIDI message. */
    fn handle_midi_message(&mut self, msg: MidiMessage) {
        let channel = msg.mtype & 0x0F;
        let mtype: u8 = msg.mtype & 0xF0;
        match mtype {
            0x90 => {
                let freq = self.keymap[msg.param as usize];
                self.voice.set_freq(freq);
                self.voice.trigger();
                self.voices_triggered += 1;
            }
            0x80 => {
                self.voices_triggered -= 1;
                if self.voices_triggered == 0 {
                    self.voice.release();
                }
            }
            _ => ()
        }
    }
}
