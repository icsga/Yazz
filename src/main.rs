mod complex_sine_osc;
mod engine;
mod oscillator;
mod sine_oscillator;
mod voice;

use oscillator::Oscillator;
use sine_oscillator::SineOscillator;
use complex_sine_osc::ComplexSineOscillator;
use engine::Engine;

/*
struct ModulatorHandler {
}

impl ModulatorHandler {
    fn add_modulator() {
    }

    fn
}
*/

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

    let num_samples = (osc.get_sample_rate() / osc.get_freq()) as usize;
    for i in 0..num_samples {
        file.write_fmt(format_args!("{:.*}\n", 5, osc.get_sample(i as f32))).unwrap();
    }
}
*/

fn main() -> Result<(), failure::Error> {
    let mut engine = Engine::new();

    //let osc1 = Box::new(SineOscillator::new(engine.get_sample_rate())); 
    let osc2 = Box::new(ComplexSineOscillator::new(engine.get_sample_rate())); 

    //engine.add_oscillator(osc1);
    engine.add_oscillator(osc2);

    engine.run()

    //test_oscillator(&mut *osc1);
}
