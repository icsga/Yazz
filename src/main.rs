#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]

mod engine;
mod envelope;
mod midi_handler;
mod multi_oscillator;
mod oscillator;
mod parameter;
//mod sine_oscillator;
//mod triangle_oscillator;
mod sample_generator;
//mod square_oscillator;
mod synth;
mod termion_wrapper;
mod tui;
mod voice;

use engine::Engine;
use envelope::Envelope;
use midi_handler::MidiHandler;
use midi_handler::MidiMessage;
use midi_handler::MessageType;
use oscillator::Oscillator;
use parameter::SynthParam;
use sample_generator::SampleGenerator;
use multi_oscillator::MultiOscillator;
//use sine_oscillator::SineOscillator;
//use triangle_oscillator::TriangleOscillator;
//use square_oscillator::SquareOscillator;
//use voice::Voice;
use synth::Synth;
use termion_wrapper::TermionWrapper;
use tui::Tui;

use std::sync::{Arc, Mutex};
//use std::sync::mpsc::{channel, Sender, Receiver};

use std::error::Error;
use std::fs::File;
use std::io::{stdin, stdout, Write};
use std::io::prelude::*;
use std::path::Path;

extern crate midir;
use midir::{MidiInput, Ignore};

#[macro_use]
extern crate crossbeam_channel;
use crossbeam_channel::unbounded;
use crossbeam_channel::{Sender, Receiver};

fn test_oscillator() {
    let mut osc = MultiOscillator::new(44100);
    let path = Path::new("osc_output.txt");
    let display = path.display();

    // Open a file in write-only mode, returns `io::Result<File>`
    let mut file = match File::create(&path) {
        Err(why) => panic!("couldn't create {}: {}", display, why.description()),
        Ok(file) => file,
    };

    let freq = 440.0;
    let num_samples = ((44000.0 / freq) * 2.0) as usize;
    for i in 0..num_samples {
        file.write_fmt(format_args!("{:.*}\n", 5, osc.get_sample(freq, i as u64))).unwrap();
    }
}

fn test_envalope() {
    let sample_rate = 44100;
    let env = Envelope::new(sample_rate);
    let path = Path::new("env_output.txt");
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
    //test_oscillator()
    let (m2s_sender, m2s_receiver) = unbounded::<MidiMessage>(); // MIDI to Synth
    let (u2s_sender, u2s_receiver) = unbounded::<SynthParam>(); // UI to Synth
    let (s2u_sender, s2u_receiver) = unbounded::<SynthParam>(); // Synth to UI

    println!("Setting up MIDI... ");
    let input = String::new();
    let mut midi_in = MidiInput::new("midir reading input").unwrap();
    midi_in.ignore(Ignore::None);
    println!("Available MIDI ports: {}", midi_in.port_count());
    let in_port = 1;
    println!("Opening connection");
    let in_port_name = midi_in.port_name(in_port).unwrap();
    let _conn_in = midi_in.connect(in_port, "midir-read-input", move |stamp, message, _| {
        if message.len() == 3 {
            let m = MidiMessage{mtype: message[0], param: message[1], value: message[2]};
            m2s_sender.send(m).unwrap();
        }
    }, ()).unwrap();
    println!("... finished.");

    //setup_ui(u2s_sender, s2u_receiver);
    println!("Setting up UI...");
    let tui = Tui::new(u2s_sender, s2u_receiver);
    let termion = TermionWrapper::new(tui);
    let term_handle = TermionWrapper::run(termion);
    println!("\r... finished");

    //setup_sound(s2u_sender, u2s_receiver).unwrap();
    println!("\rSetting up sound...");
    let mut engine = Engine::new();
    let sample_rate = engine.get_sample_rate();
    println!("\rsample_rate: {}", sample_rate);

    let synth = Synth::new(sample_rate);
    let synth = Arc::new(Mutex::new(synth));
    let synth_handle = Synth::run(synth.clone(), s2u_sender, u2s_receiver, m2s_receiver);

    println!("... finished, starting loop");

    engine.run(synth).unwrap();
    //test_envalope();


    //Ok(())
    term_handle.join().unwrap();
    synth_handle.join().unwrap();
}
