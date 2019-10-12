#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]

extern crate termion;

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
use termion::event::Key;
use tui::Tui;

use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
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
    let env = Envelope::new(sample_rate as f32);
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

pub enum SynthMessage {
    Midi(MidiMessage),
    Param(SynthParam),
}

pub enum UiMessage {
    Midi(MidiMessage),
    Key(Key),
    Param(SynthParam),
}

fn setup_messaging() -> (Sender<UiMessage>, Receiver<UiMessage>, Sender<SynthMessage>, Receiver<SynthMessage>) {
    // Prepare communication channels
    let (to_ui_sender, ui_receiver) = unbounded::<UiMessage>(); // MIDI and Synth to UI
    let (to_synth_sender, synth_receiver) = unbounded::<SynthMessage>(); // MIDI and UI to Synth
    (to_ui_sender, ui_receiver, to_synth_sender, synth_receiver)
}

fn setup_midi(m2s_sender: Sender<SynthMessage>, m2u_sender: Sender<UiMessage>) -> MidiInputConnection<()> {
    println!("Setting up MIDI... ");
    let conn_in = MidiHandler::run(m2s_sender, m2u_sender);
    println!("... finished.");
    conn_in
}

fn setup_ui(to_synth_sender: Sender<SynthMessage>, to_ui_sender: Sender<UiMessage>, ui_receiver: Receiver<UiMessage>) -> (JoinHandle<()>, JoinHandle<()>) {
    println!("Setting up UI...");
    let tui = Tui::new(to_synth_sender, ui_receiver);
    let termion = TermionWrapper::new();
    let term_handle = TermionWrapper::run(termion, to_ui_sender);
    let tui_handle = Tui::run(tui);
    println!("\r... finished");
    (term_handle, tui_handle)
}

fn setup_audio() -> (Engine, u32) {
    println!("\rSetting up audio engine...");
    let engine = Engine::new();
    let sample_rate = engine.get_sample_rate();
    println!("\rsample_rate: {}", sample_rate);
    println!("\r... finished");
    (engine, sample_rate)
}

fn setup_synth(sample_rate: u32, s2u_sender: Sender<UiMessage>, synth_receiver: Receiver<SynthMessage>) -> (Arc<Mutex<Synth>>, std::thread::JoinHandle<()>) { 
    println!("\rSetting up synth engine...");
    let synth = Synth::new(sample_rate, s2u_sender);
    let synth = Arc::new(Mutex::new(synth));
    let synth_handle = Synth::run(synth.clone(), synth_receiver);
    println!("\r... finished");
    (synth, synth_handle)
}

fn main() {
    //test_oscillator();
    //return;

    // Do setup
    let (to_ui_sender, ui_receiver, to_synth_sender, synth_receiver) = setup_messaging();
    let midi_connection = setup_midi(to_synth_sender.clone(), to_ui_sender.clone());
    let (term_handle, tui_handle) = setup_ui(to_synth_sender, to_ui_sender.clone(), ui_receiver);
    let (mut engine, sample_rate) = setup_audio();
    let (synth, synth_handle) = setup_synth(sample_rate, to_ui_sender, synth_receiver);

    // Run
    println!("\r... finished, starting processing");
    engine.run(synth).unwrap();

    // Cleanup
    term_handle.join().unwrap();
    tui_handle.join().unwrap();
    synth_handle.join().unwrap();
}

