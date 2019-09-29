#![allow(dead_code)]
#![allow(unused_imports)]

mod engine;
mod envelope;
mod oscillator;
mod sine_oscillator;
mod sample_generator;
//mod square_oscillator;
mod synth;
mod tui;
mod voice;

use engine::Engine;
use envelope::Envelope;
use oscillator::Oscillator;
use sample_generator::SampleGenerator;
use sine_oscillator::SineOscillator;
//use square_oscillator::SquareWaveOscillator;
//use voice::Voice;
use synth::Synth;
use tui::Tui;

use std::sync::{Arc, Mutex};

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
    let mut env = Envelope::new(sample_rate);
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
fn setup_ui() {
    let mut tui = Tui::new();
    tui.handle_input();
}

fn setup_sound() -> Result<(), failure::Error> {
    let mut engine = Engine::new();
    let sample_rate = engine.get_sample_rate();
    println!("sample_rate: {}", sample_rate);

    let synth = Arc::new(Mutex::new(Synth::new(sample_rate)));

    engine.run(synth)
}

fn main() -> Result<(), failure::Error> {
    //setup_ui();
    setup_sound()
    //test_envalope();

    //test_oscillator(&mut *osc);
    //Ok(())
}
