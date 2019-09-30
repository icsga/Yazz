#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]

mod engine;
mod envelope;
mod oscillator;
mod parameter;
mod sine_oscillator;
mod sample_generator;
//mod square_oscillator;
mod synth;
mod termion_wrapper;
mod tui;
mod voice;

use engine::Engine;
use envelope::Envelope;
use oscillator::Oscillator;
use parameter::SynthParam;
use sample_generator::SampleGenerator;
use sine_oscillator::SineOscillator;
//use square_oscillator::SquareWaveOscillator;
//use voice::Voice;
use synth::Synth;
use termion_wrapper::TermionWrapper;
use tui::Tui;

use std::sync::{Arc, Mutex};
use std::sync::mpsc::{channel, Sender, Receiver};

/*
use std::error::Error;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;

fn test_oscillator(osc: &mut dyn Oscillator) {
    let path = Path::new("osc_output.txt");
    let display = path.display();

    // Open a file in write-only mode, returns `io::Result<File>`
    let mut file = match File::create(&path) {
        Err(why) => panic!("couldn't create {}: {}", display, why.description()),
        Ok(file) => file,
    };

    let num_samples = (osc.get_sample_rate() as f32 / osc.get_freq()) as usize;
    for i in 0..num_samples {
        file.write_fmt(format_args!("{:.*}\n", 5, osc.get_sample(i as u64, osc.get_freq()))).unwrap();
    }
}
*/

use std::error::Error;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;


fn test_envalope() {
    let sample_rate = 44100;
    let env = Envelope::new(sample_rate);
    let path = Path::new("osc_output.txt");
    let display = path.display();

    // Open a file in write-only mode, returns `io::Result<File>`
    let mut file = match File::create(&path) {
        Err(why) => panic!("couldn't create {}: {}", display, why.description()),
        Ok(file) => file,
    };

    let num_samples = sample_rate;
    env.trigger(0 as u64);
    for i in 0..num_samples {
        if i == 20000 {
            println!("Release");
            env.release(i as u64);
        }
        file.write_fmt(format_args!("{:.*}\n", 5, env.get_sample(i as u64))).unwrap();
    }
}

/*
fn setup_ui(sender: Sender<SynthParam>, receiver: Receiver<SynthParam>) {
    println!("Setting up UI...");
    let tui = Tui::new(sender, receiver);
    let mut termion = TermionWrapper::new(tui);
    //termion.run();
    println!("... finished");
}

fn setup_sound(sender: Sender<SynthParam>, receiver: Receiver<SynthParam>) -> Result<(), failure::Error> {
    println!("Setting up sound...");
    let mut engine = Engine::new();
    let sample_rate = engine.get_sample_rate();
    println!("sample_rate: {}", sample_rate);

    let mut synth = Synth::new(sample_rate, sender, receiver);
    synth.run();
    let synth = Arc::new(Mutex::new(synth));

    println!("... finished, starting loop");

    engine.run(synth)
}
*/

fn main() {
    let (u2s_sender, u2s_receiver) = channel::<SynthParam>(); // UI to Synth
    let (s2u_sender, s2u_receiver) = channel::<SynthParam>(); // Synth to UI

    //setup_ui(u2s_sender, s2u_receiver);
    println!("Setting up UI...");
    let tui = Tui::new(u2s_sender, s2u_receiver);
    let termion = TermionWrapper::new(tui);
    let term_handle = TermionWrapper::run(termion);
    println!("... finished");

    //setup_sound(s2u_sender, u2s_receiver).unwrap();
    println!("Setting up sound...");
    let mut engine = Engine::new();
    let sample_rate = engine.get_sample_rate();
    println!("sample_rate: {}", sample_rate);

    let synth = Synth::new(sample_rate);
    let synth = Arc::new(Mutex::new(synth));
    let synth_handle = Synth::run(synth.clone(), s2u_sender, u2s_receiver);

    println!("... finished, starting loop");

    engine.run(synth).unwrap();
    //test_envalope();


    //test_oscillator(&mut *osc);
    //Ok(())
    term_handle.join().unwrap();
    synth_handle.join().unwrap();
}
