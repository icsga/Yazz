extern crate cpal;
extern crate failure;

use cpal::traits::{DeviceTrait, EventLoopTrait, HostTrait};

use std::sync::{Arc, Mutex};

pub trait Oscillator {
    fn set_freq(&mut self, frequency: f32);
    fn set_amp(&mut self, amplitude: f32);

    fn get_sample(&mut self, sample_clock: f32) -> f32;
    fn get_freq(&self) -> f32;
}

struct SineOscillator {
    sample_rate: f32,
    freq: f32,
    amp: f32,
    last_update: f32,
    last_value: f32
}

impl SineOscillator {
    fn new(sample_rate: f32) -> SineOscillator {
        let osc = SineOscillator{sample_rate: sample_rate, freq: 440.0, amp: 0.5, last_update: 0.0, last_value: 0.0};
        osc
    }
}

impl Oscillator for SineOscillator {
    fn set_freq(&mut self, frequency: f32) {
        self.freq = frequency;
    }

    fn set_amp(&mut self, amplitude: f32) {
        self.amp = amplitude;
    }

    fn get_sample(&mut self, sample_clock: f32) -> f32 {
        if sample_clock != self.last_update {
            self.last_value = (sample_clock * self.freq * 2.0 * 3.141592 / self.sample_rate).sin() * self.amp
        }
        self.last_value
    }

    fn get_freq(&self) -> f32 {
        self.freq
    }
}

/*
struct Modulator {
}
*/

/*
struct Engine {
    sample_clock: f32,
    oscillators: Arc<Mutex<Vec<Box<dyn Oscillator>>>>
}

impl Engine {
    fn new() -> Engine {
        Engine{sample_clock: 0f32, oscillators: Arc::new(Mutex::new(Vec::new()))}
    }

    fn add_oscillator(&mut self, osc: Box<dyn Oscillator>) {
        let mut oscillators = self.oscillators.lock().unwrap();
        oscillators.push(osc);
    }

    fn get_oscillators(&mut self) -> Arc<Mutex<Vec<Box<dyn Oscillator>>>> {
        self.oscillators
    }
}
*/

fn get_sample(sample_clock: f32, oscillators: &mut Vec<Box<dyn Oscillator + Send>>) -> f32 {
    let mut value = 0.0;
    //let oscs = oscillators.lock().unwrap();
    for osc in oscillators {
        value += osc.get_sample(sample_clock);
    }
    value
}

fn run() -> Result<(), failure::Error> {
    let host = cpal::default_host();
    let device = host.default_output_device().expect("failed to find a default output device");
    let format = device.default_output_format()?;
    let event_loop = host.event_loop();
    let stream_id = event_loop.build_output_stream(&device, &format)?;
    event_loop.play_stream(stream_id.clone())?;

    let sample_rate = format.sample_rate.0 as f32;
    let mut sample_clock = 0f32;

    let oscillators: Arc<Mutex<Vec<Box<dyn Oscillator + Send>>>> = Arc::new(Mutex::new(Vec::new()));
    {
        let osc_clone = oscillators.clone();
        let mut oscs = osc_clone.lock().unwrap();
        oscs.push(Box::new(SineOscillator::new(sample_rate)));
        let mut osc2 = Box::new(SineOscillator::new(sample_rate)); 
        osc2.set_freq(680.0);
        oscs.push(osc2);
    }

    //let mut osc = SineOscillator::new(sample_rate);

    event_loop.run(move |id, result| {
        let data = match result {
            Ok(data) => data,
            Err(err) => {
                eprintln!("an error occurred on stream {:?}: {}", id, err);
                return;
            }
        };

        match data {
            cpal::StreamData::Output { buffer: cpal::UnknownTypeOutputBuffer::F32(mut buffer) } => {
                let mut my_oscs = oscillators.lock().unwrap();
                for sample in buffer.chunks_mut(format.channels as usize) {
                    // Current buffer size: 512
                    sample_clock = (sample_clock + 1.0) % sample_rate;
                    let value = get_sample(sample_clock, &mut my_oscs);
                    for out in sample.iter_mut() {
                        *out = value;
                    }
                }
            },
            _ => (),
        }
    });
}

fn main() -> Result<(), failure::Error> {
    run()
}
