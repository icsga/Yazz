use super::parameter::{Function, FunctionId, Parameter, ParameterValue, SynthParam};
use super::voice::Voice;

use std::sync::{Arc, Mutex};
use std::sync::mpsc::{Sender, Receiver};
use std::thread::spawn;

pub struct Synth {
    sample_rate: u32,
    voice: Voice,
}

impl Synth {
    pub fn new(sample_rate: u32) -> Self {
        //let voices = [Voice::new(sample_rate); 12];
        let voice = Voice::new(sample_rate);
        Synth{sample_rate, voice}
    }

    pub fn run(synth: Arc<Mutex<Synth>>, sender: Sender<SynthParam>, receiver: Receiver<SynthParam>) -> std::thread::JoinHandle<()> {
        let handler = spawn(move || {
            let mut triggered: bool = false;
            loop {
                println!("Synth: Waiting for param");
                let param = receiver.recv();
                println!("Synth: Got parameter");
                let mut locked_synth = synth.lock().unwrap();
                if triggered {
                    locked_synth.voice.release();
                    triggered = false;
                } else {
                    locked_synth.voice.trigger();
                    triggered = true;
                }
            }
        });
        handler
    }

    pub fn get_sample(&mut self, sample_clock: u64) -> f32 {
        self.voice.get_sample(sample_clock)
    }
}
