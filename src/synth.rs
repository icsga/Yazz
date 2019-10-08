use super::midi_handler::MidiMessage;
use super::midi_handler::MessageType;
use super::parameter::{FunctionId, Parameter, ParameterValue, SynthParam};
use super::voice::Voice;

use std::sync::{Arc, Mutex};
use std::thread::spawn;

//use std::sync::mpsc::{Sender, Receiver};
use crossbeam_channel::unbounded;
use crossbeam_channel::{Sender, Receiver};

pub struct Synth {
    sample_rate: u32,
    voice: Voice,
    keymap: [f32; 127],
    triggered: bool,
    voices_triggered: u32,
}

impl Synth {
    pub fn new(sample_rate: u32) -> Self {
        //let voices = [Voice::new(sample_rate); 12];
        let voice = Voice::new(sample_rate);
        let mut keymap: [f32; 127] = [0.0; 127];
        Synth::calculate_keymap(&mut keymap, 440.0);
        let triggered = false;
        let voices_triggered = 0;
        Synth{sample_rate, voice, keymap, triggered, voices_triggered}
    }

    pub fn run(synth: Arc<Mutex<Synth>>, sender: Sender<SynthParam>, t2s_receiver: Receiver<SynthParam>, m2s_receiver: Receiver<MidiMessage>) -> std::thread::JoinHandle<()> {
        let handler = spawn(move || {
            loop {
                select! {
                    recv(t2s_receiver) -> msg => {
                        let mut locked_synth = synth.lock().unwrap();
                        locked_synth.handle_tui_message(msg.unwrap());
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

    pub fn get_sample(&mut self, sample_clock: u64) -> f32 {
        self.voice.get_sample(sample_clock)
    }

    fn calculate_keymap(map: &mut[f32; 127], reference_pitch: f32) {
        for i in 0..127 {
           map[i] = (reference_pitch / 32.0) * (2.0f32.powf((i as f32 - 9.0) / 12.0));
        }
    }

    fn handle_tui_message(&mut self, msg: SynthParam) {
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
