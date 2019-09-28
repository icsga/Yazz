mod complex_sine_osc;
mod engine;
mod oscillator;
mod sine_oscillator;
mod square_oscillator;
mod voice;

use oscillator::Oscillator;
use sine_oscillator::SineOscillator;
use complex_sine_osc::ComplexSineOscillator;
use square_oscillator::SquareWaveOscillator;
use engine::Engine;
use voice::Voice;

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

fn main() -> Result<(), failure::Error> {
    let mut engine = Engine::new();
    let sample_rate = engine.get_sample_rate();
    println!("sample_rate: {}", sample_rate);

    let mut voice1 = Box::new(Voice::new(sample_rate));
    let mut osc = Box::new(SquareWaveOscillator::new(sample_rate));
    let mut lfo1 = Box::new(SineOscillator::new(sample_rate));
    lfo1.set_freq(1.0);
    voice1.set_oscillator(osc);
    voice1.add_freq_mod(lfo1);
    engine.add_voice(voice1);

    engine.run()

    //test_oscillator(&mut *osc);
    //Ok(())
}
