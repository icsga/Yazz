#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(unreachable_code)]

mod midi_handler;
mod modulation;
mod parameter;
mod ringbuffer;
mod sound;
mod storage;
mod synth;
mod tui;

use synth::*;
use tui::*;

use canvas::{Canvas, CanvasRef};
use midi_handler::{MidiHandler, MidiMessage};
use modulation::{Modulator, ModData};
use parameter::*;
use ringbuffer::Ringbuffer;
use sound::SoundData;
use storage::{SoundBank, SoundPatch};
use synth::*;
use tui::Index;
use termion_wrapper::TermionWrapper;
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


pub const SYNTH_ENGINE_VERSION: &'static str = "0.0.2";
pub const SOUND_DATA_VERSION: &'static str = "0.0.1";

type Float = f32;

pub enum SynthMessage {
    Midi(MidiMessage),
    Param(SynthParam),
    Sound(SoundData),
    SampleBuffer(Vec<Float>, SynthParam),
}

pub enum UiMessage {
    Midi(MidiMessage),
    Key(Key),
    MousePress{x: Index, y: Index},
    MouseHold{x: Index, y: Index},
    MouseRelease{x: Index, y: Index},
    //Param(SynthParam),
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
    let termion = TermionWrapper::new();
    let term_handle = TermionWrapper::run(termion, to_ui_sender);
    let tui_handle = Tui::run(to_synth_sender, ui_receiver);
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

/*
fn save_wave() -> std::io::Result<()> {
    let mut osc = MultiOscillator::new(44100, 0);
    let mut data = SoundData::new();
    data.init();
    data.osc[0].select_wave(1);
    data.osc[0].num_voices = 1;
    data.osc[0].level = 1.0;
    let mut file = File::create("synth_data.csv")?;
    file.write_all(b"time,sample\n")?;
    for i in 0..441 {
        let (sample, reset) = osc.get_sample(100.0, i, &data, false);
        let s = format!("{}, {:?}\n", i, sample);
        file.write_all(s.as_bytes())?;
    }
    Ok(())
}

fn save_wave() -> std::io::Result<()> {
    let osc = WtOsc::new(44100, 0);
    let mut file = File::create("synth_data.csv")?;
    let mut table = [0.0; 2097];
    WtOsc::insert_saw(&mut table, 10.0, 44100.0);
    for i in 0..table.len() {
        let s = format!("{}, {:?}\n", i, table[i]);
        file.write_all(s.as_bytes())?;
    }
    Ok(())
}
*/

fn main() {
    setup_logging();
    //save_wave().unwrap();

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

