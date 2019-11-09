extern crate cpal;
extern crate failure;

use super::UiMessage;
use super::synth::Synth;
use cpal::traits::{DeviceTrait, EventLoopTrait, HostTrait};

use crossbeam_channel::Sender;

use std::sync::{Arc, Mutex};
use std::time::{SystemTime, Duration};

use log::{info, trace, warn};

pub struct Engine {
    sample_rate: u32,
    sample_clock: i64,
    num_channels: usize,
    to_ui_sender: Sender<UiMessage>,
}

impl Engine {
    pub fn new(to_ui_sender: Sender<UiMessage>) -> Engine {
        //Engine::enumerate();
        let host = cpal::default_host();
        println!("\r  Chose host {:?}", host.id());
        let device = host.default_output_device().expect("failed to find a default output device");
        println!("\r  Chose device {:?}", device.name());
        let format = device.default_output_format().unwrap();
        let sample_rate = format.sample_rate.0;
        let sample_clock = 0i64;
        let num_channels = 2;

        Engine{sample_rate, sample_clock, num_channels, to_ui_sender}
    }

    pub fn get_sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub fn run(&mut self, synth: Arc<Mutex<Synth>>) -> Result<(), failure::Error> {
        let host = cpal::default_host();
        let device = host.default_output_device().expect("failed to find a default output device");
        let event_loop = host.event_loop();
        let mut format = device.default_output_format().unwrap();
        if format.channels > self.num_channels as u16 {
            format.channels = self.num_channels as u16;
        }
        let stream_id = event_loop.build_output_stream(&device, &format).unwrap();
        let my_synth = synth.clone();
        let mut time = SystemTime::now();
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
                    let locked_synth = &mut synth.lock().unwrap();

                    let idle = time.elapsed().expect("Went back in time");
                    time = SystemTime::now();

                    for sample in buffer.chunks_mut(self.num_channels) {
                        self.sample_clock = self.sample_clock + 1;
                        let value = locked_synth.get_sample(self.sample_clock);
                        for out in sample.iter_mut() {
                            *out = value;
                        }
                    }

                    let busy = time.elapsed().expect("Went back in time");
                    time = SystemTime::now();
                    self.to_ui_sender.send(UiMessage::EngineSync(idle, busy)).unwrap();

                    locked_synth.update(); // Update the state of the synth voices
                },
                _ => (),
            }
        });
    }

    fn enumerate() {
        println!("\rSupported hosts:\n\r  {:?}", cpal::ALL_HOSTS);
        let available_hosts = cpal::available_hosts();
        println!("\rAvailable hosts:\n\r  {:?}", available_hosts);

        for host_id in available_hosts {
            println!("\r{:?}", host_id);
            let host = cpal::host_from_id(host_id).unwrap();
            let default_in = host.default_input_device().map(|e| e.name().unwrap());
            let default_out = match host.default_output_device().map(|e| e.name()) {
                Some(n) => match n {
                    Ok(s) => s,
                    Err(e) => "<unknown>".to_string(),
                }
                None => "<unknown>".to_string(),
            };
            println!("\r  Default Input Device:\n\r    {:?}", default_in);
            println!("\r  Default Output Device:\n\r    {:?}", default_out);

            let devices = host.devices().unwrap();
            println!("\r  Devices: ");
            for (device_index, device) in devices.enumerate() {
                let name = match device.name() {
                    Ok(n) => n,
                    Err(e) => "Unknown".to_string(),
                };
                println!("\r  {}. \"{}\"", device_index + 1, name);

                // Input formats
                if let Ok(fmt) = device.default_input_format() {
                    println!("\r    Default input stream format:\n\r      {:?}", fmt);
                }
                let mut input_formats = match device.supported_input_formats() {
                    Ok(f) => f.peekable(),
                    Err(e) => {
                        println!("Error: {:?}", e);
                        continue;
                    },
                };
                if input_formats.peek().is_some() {
                    println!("\r    All supported input stream formats:");
                    for (format_index, format) in input_formats.enumerate() {
                        println!("\r      {}.{}. {:?}", device_index + 1, format_index + 1, format);
                    }
                }

                // Output formats
                if let Ok(fmt) = device.default_output_format() {
                    println!("\r    Default output stream format:\n\r      {:?}", fmt);
                }
                let mut output_formats = match device.supported_output_formats() {
                    Ok(f) => f.peekable(),
                    Err(e) => {
                        println!("Error: {:?}", e);
                        continue;
                    },
                };
                if output_formats.peek().is_some() {
                    println!("\r    All supported output stream formats:");
                    for (format_index, format) in output_formats.enumerate() {
                        println!("\r      {}.{}. {:?}", device_index + 1, format_index + 1, format);
                    }
                }
            }
        }
    }
}

