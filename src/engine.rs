extern crate cpal;
extern crate failure;

use super::oscillator::Oscillator;
use cpal::traits::{DeviceTrait, EventLoopTrait, HostTrait};
use std::sync::{Arc, Mutex};

pub struct Engine {
    sample_rate: u32,
    sample_clock: u64,
    oscillators: Arc<Mutex<Vec<Box<dyn Oscillator + Send>>>>,
    num_channels: usize,
}

impl Engine {
    pub fn new() -> Engine {
        let host = cpal::default_host();
        let device = host.default_output_device().expect("failed to find a default output device");
        let format = device.default_output_format().unwrap();

        let sample_rate = format.sample_rate.0;
        let sample_clock = 0u64;

        let oscillators: Arc<Mutex<Vec<Box<dyn Oscillator + Send>>>> = Arc::new(Mutex::new(Vec::new()));
        let num_channels: usize = format.channels as usize;

        Engine{sample_rate, sample_clock, oscillators, num_channels}
    }

    pub fn add_oscillator(&mut self, osc: Box<dyn Oscillator + Send>) {
        let mut oscillators = self.oscillators.lock().unwrap();
        oscillators.push(osc);
    }

    pub fn get_sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn get_sample(&mut self) -> f32 {
        let mut value = 0.0;
        let mut oscs_mutex = self.oscillators.lock().unwrap();
        for osc in oscs_mutex.iter_mut() {
            value += osc.get_sample(self.sample_clock, 440.0);
        }
        value
    }

    pub fn run(&mut self) -> Result<(), failure::Error> {
        let host = cpal::default_host();
        let device = host.default_output_device().expect("failed to find a default output device");
        let format = device.default_output_format().unwrap();
        let event_loop = host.event_loop();
        let stream_id = event_loop.build_output_stream(&device, &format).unwrap();
        event_loop.play_stream(stream_id.clone()).unwrap();

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
                    for sample in buffer.chunks_mut(self.num_channels) {
                        // Current buffer size: 512
                        self.sample_clock = self.sample_clock + 1;
                        let value = self.get_sample();
                        for out in sample.iter_mut() {
                            *out = value;
                        }
                    }
                },
                _ => (),
            }
        });
    }
}


