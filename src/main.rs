//! Yazz - Yet another subtractive synthesizer
//!
//! # Running the synth
//!
//! The synth needs to find a MIDI interface and a soundcard in the system.
//! For sound, the default output device is used. The MIDI device to use as
//! input can be selected with the "-m <ID>" command line parameter.
//!
//! # Running the tests
//!
//! The test code supports writing output to a logfile. Since only a single
//! logfile writer can exist, the tests currently can't run in parallel. The
//! easiest way around this problem is by starting the test with a single
//! thread:
//! > RUST_TEST_THREADS=1 cargo test
//!

#![allow(dead_code)]
//#![allow(unused_imports)]
//#![allow(unused_variables)]
//#![allow(unreachable_code)]

mod ctrl_map;
use ctrl_map::{CtrlMap, MappingType};

mod midi_handler;
use midi_handler::{MidiHandler, MidiMessage};

mod modulation;
use modulation::ModData;

mod parameter;
use parameter::*;

mod sound;
use sound::SoundData;

mod storage;
use storage::{SoundBank, SoundPatch};

mod synth;
use synth::*;
use voice::Voice;

mod tui;
use tui::{Tui, Index};
use tui::termion_wrapper::TermionWrapper;

mod value_range;
use value_range::ValueRange;

extern crate termion;
use termion::event::Key;

extern crate midir;
use midir::MidiInputConnection;

extern crate crossbeam_channel;
use crossbeam_channel::unbounded;
use crossbeam_channel::{Sender, Receiver};

use flexi_logger::{Logger, opt_format};
use log::error;

extern crate clap;
use clap::{Arg, App};

extern crate wavetable;
use wavetable::{WtInfo, WtManager};

use std::io::prelude::*;
use std::fs::File;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;
use std::vec::Vec;

pub const SYNTH_ENGINE_VERSION: &'static str = "0.0.7";
pub const SOUND_DATA_VERSION: &'static str = "0.0.7";

type Float = f64;

// Messages sent to the synth engine
pub enum SynthMessage {
    Midi(MidiMessage),
    Param(SynthParam),
    Sound(SoundData),
    Wavetable(WtInfo),
    SampleBuffer(Vec<Float>, SynthParam),
    Exit
}

// Messages sent to the UI
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

fn setup_midi(m2s_sender: Sender<SynthMessage>, m2u_sender: Sender<UiMessage>, midi_port: usize, mut midi_channel: u8) -> Result<MidiInputConnection<()>, ()> {
    println!("Setting up MIDI... ");
    if midi_channel < 1 || midi_channel > 16 {
        midi_channel = 16; // Omni
    } else {
        midi_channel -= 1; // 0 - 15
    }
    let conn_in = MidiHandler::run(m2s_sender, m2u_sender, midi_port, midi_channel);
    println!("... finished.");
    conn_in
}

fn setup_ui(to_synth_sender: Sender<SynthMessage>, to_ui_sender: Sender<UiMessage>, ui_receiver: Receiver<UiMessage>, show_tui: bool) -> Result<(JoinHandle<()>, JoinHandle<()>), ()> {
    println!("Setting up UI...");
    let termion_result = TermionWrapper::new();
    let termion = match termion_result {
        Ok(t) => t,
        Err(e) => {
            println!("\nError setting terminal into raw mode: {}", e);
            return Err(());
        }
    };
    let term_handle = TermionWrapper::run(termion, to_ui_sender);
    let tui_handle = Tui::run(to_synth_sender, ui_receiver, show_tui);
    println!("\r... finished");
    Ok((term_handle, tui_handle))
}

fn setup_synth(sample_rate: u32, s2u_sender: Sender<UiMessage>, synth_receiver: Receiver<SynthMessage>) -> (Arc<Mutex<Synth>>, std::thread::JoinHandle<()>) { 
    println!("\rSetting up synth engine...");
    let synth = Synth::new(sample_rate, s2u_sender);
    let synth = Arc::new(Mutex::new(synth));
    let synth_handle = Synth::run(synth.clone(), synth_receiver);
    println!("\r... finished");
    (synth, synth_handle)
}

fn setup_audio() -> Result<(Engine, u32), ()> {
    println!("\rSetting up audio engine...");
    let result = Engine::new();
    let engine = match result {
        Ok(e) => e,
        Err(()) => {
            error!("Failed to start audio engine");
            println!("Failed to start audio engine");
            return Err(());
        }
    };
    let sample_rate = engine.get_sample_rate();
    println!("\r  sample_rate: {}", sample_rate);
    println!("\r... finished");
    Ok((engine, sample_rate))
}

// Save one table of a wavetable set as a CSV file.
fn save_wave(id: usize) -> std::io::Result<()> {
    let wt_manager = WtManager::new(44100.0, ".");
    let mut filename = "synth_wave_".to_string();
    filename += &id.to_string();
    filename += ".csv";
    let mut file = File::create(filename)?;
    let wt = wt_manager.get_table(0).unwrap();
    let t = &wt.table[id];
    // Write only the first octave table (= first 2048 values)
    for i in 0..2048 {
        let s = format!("{}, {:?}\n", i, t[i]);
        file.write_all(s.as_bytes())?;
    }
    Ok(())
}

// Save one samplebuffer of a voice as CSV file.
fn save_voice() -> std::io::Result<()> {
    let wt_manager = WtManager::new(44100.0, ".");
    let filename = "synth_voice.csv".to_string();
    let mut file = File::create(filename)?;
    let wt = wt_manager.get_table(0).unwrap();
    let mut voice = Voice::new(44100, wt);
    let mut sound_global = SoundData::new();
    let mut sound_local = SoundData::new();
    let global_state = SynthState{freq_factor: 1.0};

    sound_global.init();
    sound_global.osc[0].level = 1.0;
    sound_global.env[0].attack = 0.0;
    sound_global.env[0].decay = 0.0;
    sound_global.env[0].sustain = 1.0;
    sound_global.env[0].factor = 1.0;
    sound_global.filter[0].filter_type = 0; // Bypass

    voice.set_freq(21.533203125);
    voice.trigger(0, 0, &sound_global);

    for i in 0..2048 {
        let value = voice.get_sample(i, &mut sound_global, &mut sound_local, &global_state);
        let s = format!("{}, {:?}\n", i, value);
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
                        .arg(Arg::with_name("version")
                            .short("v")
                            .long("version")
                            .help("Shows the version of the sound engine and the sound file format"))
                        .arg(Arg::with_name("notui")
                            .short("n")
                            .long("no-tui")
                            .help("Disable drawing of text user interface"))
                        .arg(Arg::with_name("savewave")
                            .short("s")
                            .long("save")
                            .help("Saves selected wave to file")
                            .takes_value(true))
                        .arg(Arg::with_name("savevoice")
                            .short("o")
                            .long("voice")
                            .help("Saves output of single voice to file"))
                        .arg(Arg::with_name("midiport")
                            .short("m")
                            .long("midiport")
                            .help("Selects the MIDI port to receive MIDI events on (1 - n, default 1)")
                            .takes_value(true))
                        .arg(Arg::with_name("midichannel")
                            .short("c")
                            .long("midichannel")
                            .help("Selects the MIDI channel to receive MIDI events on (1 - 16, default = omni)")
                            .takes_value(true))
                        .get_matches();
    let midi_port = matches.value_of("midiport").unwrap_or("1");
    let midi_port: usize = midi_port.parse().unwrap_or(1);
    let midi_channel = matches.value_of("midichannel").unwrap_or("0");
    let midi_channel: u8 = midi_channel.parse().unwrap_or(0);
    let show_tui = !matches.is_present("notui");

    // Show version
    if matches.is_present("version") {
        println!("Yazz v{}, sound file v{}", SYNTH_ENGINE_VERSION, SOUND_DATA_VERSION);
        return;
    }

    // For debugging: Save selected wavetable as file
    let wave_index = matches.value_of("savewave").unwrap_or("");
    if wave_index.len() > 0 {
        let wave_index: usize = wave_index.parse().unwrap_or(1);
        save_wave(wave_index).unwrap();
        return;
    }
    if matches.is_present("savevoice") {
        println!("Saving voice output");
        save_voice().unwrap();
        return;
    }

    // Do setup
    let (to_ui_sender, ui_receiver, to_synth_sender, synth_receiver) = setup_messaging();
    let result = setup_midi(to_synth_sender.clone(), to_ui_sender.clone(), midi_port, midi_channel);
    let midi_connection = match result {
        Ok(c) => c,
        Err(()) => return,
    };

    let result = setup_audio();
    let (mut engine, sample_rate) = match result {
        Ok((e, s)) => (e, s),
        Err(()) => return,
    };

    let result = setup_ui(to_synth_sender, to_ui_sender.clone(), ui_receiver, show_tui);
    let (term_handle, tui_handle) = match result {
        Ok((term, tui)) => (term, tui),
        Err(_) => return, // TODO: Reset terminal to non-raw state
    };

    let (synth, synth_handle) = setup_synth(sample_rate, to_ui_sender.clone(), synth_receiver);

    // Run
    println!("\r... finished, starting processing");
    engine.run(synth, to_ui_sender).unwrap();

    // Cleanup
    term_handle.join().unwrap();
    println!("\rTerminal handler finished");
    midi_connection.close();
    tui_handle.join().unwrap();
    println!("TUI finished");
    synth_handle.join().unwrap();
    println!("Synth engine finished");
}

