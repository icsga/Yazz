use midir::{MidiInput, MidiInputConnection, Ignore};

use crossbeam_channel::Sender;
use log::{info, error};

use super::{SynthMessage, UiMessage};
use super::Float;

#[derive(Clone, Copy, Debug)]
pub enum MidiMessage {
    NoteOff    {channel: u8, key: u8, velocity: u8},
    NoteOn     {channel: u8, key: u8, velocity: u8},
    KeyAT      {channel: u8, key: u8, pressure: u8},
    ControlChg {channel: u8, controller: u8, value: u8},
    ProgramChg {channel: u8, program: u8},
    ChannelAT  {channel: u8, pressure: u8},
    Pitchbend  {channel: u8, pitch: i16},
    SongPos    {position: u16},
    TimingClock,
    Start,
    Continue,
    Stop,
    ActiveSensing,
    Reset,
}

pub struct MidiHandler {
    last_timestamp: u64,
    bpm: Float
}

impl MidiHandler {
    fn new() -> Self {
        MidiHandler{last_timestamp: 0, bpm: 0.0}
    }

    /** Starts the thread for receiving MIDI events.
     *
     * If the midi_channel argument is 0, all events are forwarded. If it is
     * between 1 and 16, only events arriving on that channel are forwarded.
     */
    pub fn run(m2s_sender: Sender<SynthMessage>,
               m2u_sender: Sender<UiMessage>,
               midi_port: usize,
               midi_channel: u8) -> Result<MidiInputConnection<()>, ()> {
        let result = MidiInput::new("Yazz MIDI input");
        let mut midi_in = match result {
            Ok(m) => m,
            Err(e) => {
                error!("Can't open MIDI input connection: {:?}", e);
                println!("Can't open MIDI input connection: {:?}", e);
                return Err(());
            }
        };
        midi_in.ignore(Ignore::None);
        let result = midi_in.port_name(midi_port);
        let in_port_name = match result {
            Ok(n) => n,
            Err(e) => {
                error!("Can't get MIDI port name: {:?}", e);
                println!("Can't get MIDI port name: {:?}", e);
                return Err(());
            }
        };
        let mut mh = MidiHandler::new();
        info!("  Connecting to MIDI port {}", in_port_name);
        println!("  Connecting to MIDI port {}", in_port_name);
        let conn_result = midi_in.connect(midi_port, "midir-read-input", move |timestamp, message, _| {
            if midi_channel < 16 && (message[0] & 0x0F) != midi_channel {
                return;
            }
            let m = MidiHandler::get_midi_message(message);
            info!("MidiMessage: {:?}", m);
            match m {
                MidiMessage::ControlChg{channel: _, controller: _, value: _} |
                MidiMessage::ProgramChg{channel: _, program: _} => {
                    // Send control change and program change to UI
                    m2u_sender.send(UiMessage::Midi(m)).unwrap();
                }
                MidiMessage::TimingClock => {
                    // Calculate BPM
                    let bpm_changed = mh.calc_bpm(timestamp);
                    if bpm_changed {
                        m2s_sender.send(SynthMessage::Bpm(mh.bpm)).unwrap();
                    }
                }
                _ => {
                    // Send everything else directly to the synth engine
                    m2s_sender.send(SynthMessage::Midi(m)).unwrap();
                }
            }
        }, ());
        match conn_result {
            Ok(c) => Ok(c),
            Err(e) => {
                error!("Failed to connect to MIDI port: {:?}", e);
                Err(())
            }
        }
    }

    pub fn get_midi_message(message: &[u8]) -> MidiMessage {
        let param = if message.len() > 1 { message[1] } else { 0 };
        let value = if message.len() > 2 { message[2] } else { 0 };

        match message[0] {
            0xF2 => {
                let mut position: u16 = param as u16;
                position |= (value as u16) << 7;
                MidiMessage::SongPos{position}
            }
            0xF8 => MidiMessage::TimingClock,
            0xFA => MidiMessage::Start,
            0xFB => MidiMessage::Continue,
            0xFC => MidiMessage::Stop,
            0xFE => MidiMessage::ActiveSensing,
            0xFF => MidiMessage::Reset,
            _ => {
                let channel = message[0] & 0x0F;
                match message[0] & 0xF0 {
                    0x90 => MidiMessage::NoteOn{channel, key: param, velocity: value},
                    0x80 => MidiMessage::NoteOff{channel, key: param, velocity: value},
                    0xA0 => MidiMessage::KeyAT{channel, key: param, pressure: value},
                    0xB0 => MidiMessage::ControlChg{channel, controller: param, value},
                    0xC0 => MidiMessage::ProgramChg{channel, program: param},
                    0xD0 => MidiMessage::ChannelAT{channel, pressure: param},
                    0xE0 => {
                        let mut pitch: i16 = param as i16;
                        pitch |= (value as i16) << 7;
                        pitch -= 0x2000;
                        MidiMessage::Pitchbend{channel, pitch}
                    },
                    _ => panic!(),
                }
            }
        }
    }

    fn calc_bpm(&mut self, timestamp: u64) -> bool {
        let mut bpm_changed = false;
        if self.last_timestamp != 0 {
            // We have a previous TS, so we can calculate the current BPM
            let diff = (timestamp - self.last_timestamp) * 24; // Diff is in usec
            let bpm = 60000000.0 / diff as f64;
            //let bpm = self.avg.add_value(bpm);
            // Calculate up to 1 decimal of BPM
            let bpm = (bpm * 10.0).round() / 10.0;
            if bpm != self.bpm {
                self.bpm = bpm;
                bpm_changed = true;
            }
        }
        self.last_timestamp = timestamp;
        bpm_changed
    }

}
