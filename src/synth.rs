use super::midi_handler::MidiMessage;
use super::parameter::{Function, FunctionId, Parameter, ParameterValue, SynthParam};
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
}

impl Synth {
    pub fn new(sample_rate: u32) -> Self {
        //let voices = [Voice::new(sample_rate); 12];
        let voice = Voice::new(sample_rate);
        let mut keymap: [f32; 127] = [0.0; 127];
        Synth::calculate_keymap(&mut keymap, 440.0);
        Synth{sample_rate, voice, keymap}
    }

    pub fn run(synth: Arc<Mutex<Synth>>, sender: Sender<SynthParam>, t2s_receiver: Receiver<SynthParam>, m2s_receiver: Receiver<MidiMessage>) -> std::thread::JoinHandle<()> {
        let handler = spawn(move || {
            let mut triggered: bool = false;
            loop {
                select! {
                    recv(t2s_receiver) -> msg => {
                        let mut locked_synth = synth.lock().unwrap();
                        if triggered {
                            locked_synth.voice.release();
                            triggered = false;
                        } else {
                            locked_synth.voice.trigger();
                            triggered = true;
                        }
                    },
                    recv(m2s_receiver) -> msg => {
                        let mut locked_synth = synth.lock().unwrap();
                        let message: MidiMessage = msg.unwrap();
                        let freq = locked_synth.keymap[message.param as usize];
                        locked_synth.voice.set_freq(freq);
                        if triggered {
                            locked_synth.voice.release();
                            triggered = false;
                        } else {
                            locked_synth.voice.trigger();
                            triggered = true;
                        }
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
}
