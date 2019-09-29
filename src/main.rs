mod complex_sine_osc;
mod engine;
mod oscillator;
mod sine_oscillator;
mod square_oscillator;
mod voice;
mod synth;
mod tui;

use oscillator::Oscillator;
//use sine_oscillator::SineOscillator;
use complex_sine_osc::ComplexSineOscillator;
//use square_oscillator::SquareWaveOscillator;
use engine::Engine;
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
    setup_ui();
    setup_sound()

    //test_oscillator(&mut *osc);
    //Ok(())
}
