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
//use midi_handler::MidiHandler;
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
use synth::{Synth, Synth2UIMessage};
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
use midir::{MidiInput, MidiInputConnection, Ignore};

#[macro_use]
extern crate crossbeam_channel;
use crossbeam_channel::unbounded;
use crossbeam_channel::{Sender, Receiver};

extern crate rand;
use rand::Rng;

fn test_oscillator() {
    let mut osc = MultiOscillator::new(44100);
    let path = Path::new("osc_output.txt");
    let display = path.display();
    let modulator = MultiOscillator::new(44100);

    // Open a file in write-only mode, returns `io::Result<File>`
    let mut file = match File::create(&path) {
        Err(why) => panic!("couldn't create {}: {}", display, why.description()),
        Ok(file) => file,
    };

    //osc.set_ratios(0.5, 0.0, 0.5, 0.0);
    let freq = 440.0;
    //let num_samples = ((44000.0 / freq) * 2.0) as usize;
    let num_samples_per_wave = (44000.0 / freq) as usize;
    let num_samples = num_samples_per_wave * 4;
    /*
    for i in 0..num_samples {
        let mod_val = (modulator.get_sample(1.0, i as u64) + 1.0) * 0.5;
        osc.set_ratio(mod_val);
        file.write_fmt(format_args!("{:.*}\n", 5, osc.get_sample(freq, i as u64))).unwrap();
    }
    */

    // Plot ratios
    let step = 3.0 / num_samples as f32;
    for i in 0..num_samples {
        osc.set_ratio(i as f32 * step);
        //file.write_fmt(format_args!("{:.*}\n", 5, osc.get_sample(freq, i as u64))).unwrap();
        write!(&mut file, "{} {} {} {}\n", osc.sine_ratio, osc.tri_ratio, osc.saw_ratio, osc.square_ratio).unwrap();
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

fn setup_midi(m2s_sender: Sender<MidiMessage>) -> MidiInputConnection<()> {
    println!("Setting up MIDI... ");
    let input = String::new();
    let mut midi_in = MidiInput::new("midir reading input").unwrap();
    midi_in.ignore(Ignore::None);
    println!("Available MIDI ports: {}", midi_in.port_count());
    let in_port = 1;
    println!("Opening connection");
    let in_port_name = midi_in.port_name(in_port).unwrap();
    let conn_in = midi_in.connect(in_port, "midir-read-input", move |stamp, message, _| {
        if message.len() == 3 {
            let m = MidiMessage{mtype: message[0], param: message[1], value: message[2]};
            m2s_sender.send(m).unwrap();
        }
    }, ()).unwrap();
    println!("... finished.");
    conn_in
}

fn setup_ui(u2s_sender: Sender<SynthParam>, s2u_receiver: Receiver<Synth2UIMessage>) -> std::thread::JoinHandle<()> {
    println!("Setting up UI...");
    let tui = Tui::new(u2s_sender, s2u_receiver);
    let termion = TermionWrapper::new(tui);
    let term_handle = TermionWrapper::run(termion);
    println!("\r... finished");
    term_handle
}

fn setup_audio() -> (Engine, u32) {
    println!("\rSetting up audio engine...");
    let engine = Engine::new();
    let sample_rate = engine.get_sample_rate();
    println!("\rsample_rate: {}", sample_rate);
    println!("\r... finished");
    (engine, sample_rate)
}

fn setup_synth(sample_rate: u32, s2u_sender: Sender<Synth2UIMessage>, u2s_receiver: Receiver<SynthParam>, m2s_receiver: Receiver<MidiMessage>) -> (Arc<Mutex<Synth>>, std::thread::JoinHandle<()>) { 
    println!("\rSetting up synth engine...");
    let synth = Synth::new(sample_rate, s2u_sender);
    let synth = Arc::new(Mutex::new(synth));
    let synth_handle = Synth::run(synth.clone(), u2s_receiver, m2s_receiver);
    println!("\r... finished");
    (synth, synth_handle)
}

fn main() {
    //test_oscillator();
    //return;

    // Prepare communication channels
    let (m2s_sender, m2s_receiver) = unbounded::<MidiMessage>(); // MIDI to Synth
    let (u2s_sender, u2s_receiver) = unbounded::<SynthParam>(); // UI to Synth
    let (s2u_sender, s2u_receiver) = unbounded::<Synth2UIMessage>(); // Synth to UI

    // Do setup
    let midi_connection = setup_midi(m2s_sender);
    let term_handle = setup_ui(u2s_sender, s2u_receiver);
    let (mut engine, sample_rate) = setup_audio();
    let (synth, synth_handle) = setup_synth(sample_rate, s2u_sender, u2s_receiver, m2s_receiver);

    // Run
    println!("\r... finished, starting processing");
    engine.run(synth).unwrap();

    // Cleanup
    term_handle.join().unwrap();
    synth_handle.join().unwrap();
}

