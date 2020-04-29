#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(unreachable_code)]

mod ctrl_map;
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
use ctrl_map::{CtrlMap, MappingType};
use midi_handler::{MidiHandler, MidiMessage};
use modulation::ModData;
use parameter::*;
use ringbuffer::Ringbuffer;
use select::{SelectorState, ParamSelector, next, ItemSelection};
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

extern crate clap;
use clap::{Arg, App};

pub const SYNTH_ENGINE_VERSION: &'static str = "0.0.4";
pub const SOUND_DATA_VERSION: &'static str = "0.0.3";

type Float = f32;

pub enum SynthMessage {
    Midi(MidiMessage),
    Param(SynthParam),
    Sound(SoundData),
    SampleBuffer(Vec<Float>, SynthParam),
    Exit
}

pub enum UiMessage {
    Midi(MidiMessage),
    Key(Key),
    MousePress{x: Index, y: Index},
    MouseHold{x: Index, y: Index},
    MouseRelease{x: Index, y: Index},
    SampleBuffer(Vec<Float>, SynthParam),
    EngineSync(Duration, Duration),
    Exit,
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

fn setup_midi(m2s_sender: Sender<SynthMessage>, m2u_sender: Sender<UiMessage>, midi_port: usize) -> MidiInputConnection<()> {
    println!("Setting up MIDI... ");
    let conn_in = MidiHandler::run(m2s_sender, m2u_sender, midi_port);
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

fn setup_synth(sample_rate: u32, s2u_sender: Sender<UiMessage>, synth_receiver: Receiver<SynthMessage>) -> (Arc<Mutex<Synth>>, std::thread::JoinHandle<()>) { 
    println!("\rSetting up synth engine...");
    let synth = Synth::new(sample_rate, s2u_sender);
    let synth = Arc::new(Mutex::new(synth));
    let synth_handle = Synth::run(synth.clone(), synth_receiver);
    println!("\r... finished");
    (synth, synth_handle)
}

fn setup_audio() -> (Engine, u32) {
    println!("\rSetting up audio engine...");
    let engine = Engine::new();
    let sample_rate = engine.get_sample_rate();
    println!("\r  sample_rate: {}", sample_rate);
    println!("\r... finished");
    (engine, sample_rate)
}

/* Save one table of a wavetable set as a CSV file. */
fn save_wave(id: usize) -> std::io::Result<()> {
    let wt_manager = WtManager::new(44100.0);
    let mut filename = "synth_wave_".to_string();
    filename += &id.to_string();
    filename += ".csv";
    let mut file = File::create(filename)?;
    let wt = wt_manager.get_table("default").unwrap();
    let t = &wt.table[id];
    // Write only the first octave table (= first 2048 values)
    for i in 0..2048 {
        let s = format!("{}, {:?}\n", i, t[i]);
        file.write_all(s.as_bytes())?;
    }
    Ok(())
}

fn main() {
    setup_logging();

    // Command line arguments
    let matches = App::new("Yazz")
                        .version(SYNTH_ENGINE_VERSION)
                        .about("Yet Another Subtractive Synth")
                        .arg(Arg::with_name("savewave")
                            .short("s")
                            .long("save")
                            .help("Saves selected wave to file")
                            .takes_value(true))
                        .arg(Arg::with_name("midiport")
                            .short("m")
                            .long("midiport")
                            .help("Selects the MIDI port to receive MIDI events on")
                            .takes_value(true))
                        .get_matches();
    let midi_port = matches.value_of("midiport").unwrap_or("1");
    let midi_port: usize = midi_port.parse().unwrap_or(1);

    // For debugging: Save selected wavetable as file
    let wave_index = matches.value_of("savewave").unwrap_or("");
    if wave_index.len() > 0 {
        let wave_index: usize = wave_index.parse().unwrap_or(1);
        save_wave(wave_index).unwrap();
        return;
    }

    // Do setup
    let (to_ui_sender, ui_receiver, to_synth_sender, synth_receiver) = setup_messaging();
    let midi_connection = setup_midi(to_synth_sender.clone(), to_ui_sender.clone(), midi_port);
    let (term_handle, tui_handle) = setup_ui(to_synth_sender, to_ui_sender.clone(), ui_receiver);
    let (mut engine, sample_rate) = setup_audio();
    let (synth, synth_handle) = setup_synth(sample_rate, to_ui_sender.clone(), synth_receiver);

    // Run
    println!("\r... finished, starting processing");
    engine.run(synth, to_ui_sender);

    // Cleanup
    term_handle.join().unwrap();
    println!("\rTerminal handler finished");
    midi_connection.close();
    tui_handle.join().unwrap();
    println!("TUI finished");
    synth_handle.join().unwrap();
    println!("Synth engine finished");
}

