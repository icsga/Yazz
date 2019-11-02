#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(unreachable_code)]

mod canvas;
mod delay;
mod engine;
mod envelope;
mod filter;
mod lfo;
mod midi_handler;
mod modulation;
mod multi_oscillator;
mod oscillator;
mod parameter;
mod param_selection;
mod ringbuffer;
mod sample_generator;
mod sound;
mod synth;
mod termion_wrapper;
mod tui;
mod voice;

use canvas::Canvas;
use delay::{Delay, DelayData};
use engine::Engine;
use envelope::{Envelope, EnvelopeData};
use filter::{Filter, FilterData};
use lfo::{Lfo, LfoData};
use midi_handler::{MidiHandler, MidiMessage, MessageType};
use modulation::{Modulator, ModData};
use multi_oscillator::{MultiOscillator, MultiOscData};
use oscillator::Oscillator;
use parameter::{Parameter, ParameterValue, SynthParam, ParamId, FunctionId};
use param_selection::ParamSelection;
use ringbuffer::Ringbuffer;
use sample_generator::SampleGenerator;
use sound::SoundData;
use synth::Synth;
use termion_wrapper::TermionWrapper;
use tui::{Tui};
use voice::Voice;

use std::error::Error;
use std::fs::File;
use std::io::{stdin, stdout, Write};
use std::io::prelude::*;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, SystemTime};
use std::vec::Vec;

extern crate termion;
use termion::event::Key;

extern crate midir;
use midir::{MidiInput, MidiInputConnection, Ignore};

#[macro_use]
extern crate crossbeam_channel;
use crossbeam_channel::unbounded;
use crossbeam_channel::{Sender, Receiver};

extern crate rand;
use rand::Rng;

use log::{info, trace, warn};
use flexi_logger::{Logger, opt_format};

type Float = f32;

/*
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
    let step = 3.0 / num_samples as Float;
    for i in 0..num_samples {
        osc.set_ratio(i as Float * step);
        //file.write_fmt(format_args!("{:.*}\n", 5, osc.get_sample(freq, i as u64))).unwrap();
        write!(&mut file, "{} {} {} {}\n", osc.sine_ratio, osc.tri_ratio, osc.saw_ratio, osc.square_ratio).unwrap();
    }
}

fn test_envalope() {
    let sample_rate = 44100;
    let mut env = Envelope::new(sample_rate as Float);
    let path = Path::new("env_output.txt");
    let display = path.display();
    let mut sound = SoundData::new();
    sound.init();

    // Open a file in write-only mode, returns `io::Result<File>`
    let mut file = match File::create(&path) {
        Err(why) => panic!("couldn't create {}: {}", display, why.description()),
        Ok(file) => file,
    };

    let num_samples = sample_rate * 2;
    env.trigger(0 as i64, &sound.env[0]);
    for i in 0..num_samples {
        if i == 10000 {
            env.release(i as i64, &sound.env[0]);
        }
        if i == 20000 {
            env.trigger(i as i64, &sound.env[0]);
        }
        let now = SystemTime::now();
        let value = env.get_sample(i as i64, &sound.env[0]);
        let duration = now.elapsed().expect("Sound");
        //file.write_fmt(format_args!("{:.*} {}\n", 5, value, duration.as_nanos())).unwrap();
        file.write_fmt(format_args!("{:.*}\n", 5, value)).unwrap();
    }
}
*/

pub enum SynthMessage {
    Midi(MidiMessage),
    Param(SynthParam),
    ParamQuery(SynthParam),
    SampleBuffer(Vec<Float>, SynthParam),
}

pub enum UiMessage {
    Midi(MidiMessage),
    Key(Key),
    Param(SynthParam),
    SampleBuffer(Vec<Float>, SynthParam),
    EngineSync(Duration, Duration),
}

fn setup_logging() {
    Logger::with_env_or_str("myprog=debug, mylib=warn")
                            .log_to_file()
                            .directory("log_files")
                            .format(opt_format)
                            .start()
                            .unwrap();
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
    //tui.init();
    let tui_handle = Tui::run(tui);
    println!("\r... finished");
    (term_handle, tui_handle)
}

fn setup_audio(to_ui_sender: Sender<UiMessage>) -> (Engine, u32) {
    println!("\rSetting up audio engine...");
    let engine = Engine::new(to_ui_sender);
    let sample_rate = engine.get_sample_rate();
    println!("\r  sample_rate: {}", sample_rate);
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
    setup_logging();

    //test_oscillator();
    //return;
    //test_envalope();
    //return;

    // Do setup
    let (to_ui_sender, ui_receiver, to_synth_sender, synth_receiver) = setup_messaging();
    let midi_connection = setup_midi(to_synth_sender.clone(), to_ui_sender.clone());
    let (term_handle, tui_handle) = setup_ui(to_synth_sender, to_ui_sender.clone(), ui_receiver);
    let (mut engine, sample_rate) = setup_audio(to_ui_sender.clone());
    let (synth, synth_handle) = setup_synth(sample_rate, to_ui_sender, synth_receiver);

    // Run
    println!("\r... finished, starting processing");
    engine.run(synth).unwrap();

    // Cleanup
    term_handle.join().unwrap();
    tui_handle.join().unwrap();
    synth_handle.join().unwrap();
}

