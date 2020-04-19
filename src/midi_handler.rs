use midir::{MidiInput, MidiInputConnection, Ignore};

use crossbeam_channel::{Sender, Receiver};
use log::{info, trace, warn};

use super::{SynthMessage, UiMessage};

#[derive(Clone, Copy, Debug)]
pub enum MidiMessage {
    NoteOff    {channel: u8, key: u8, velocity: u8},
    NoteOn     {channel: u8, key: u8, velocity: u8},
    KeyAT      {channel: u8, key: u8, pressure: u8},
    ControlChg {channel: u8, controller: u8, value: u8},
    ProgramChg {channel: u8, program: u8},
    ChannelAT  {channel: u8, pressure: u8},
    PitchWheel {channel: u8, pitch: i16},
}

pub struct MidiHandler {
}

impl MidiHandler {
    pub fn run(m2s_sender: Sender<SynthMessage>, m2u_sender: Sender<UiMessage>) -> MidiInputConnection<()> {
        let input = String::new();
        let mut midi_in = MidiInput::new("midir reading input").unwrap();
        midi_in.ignore(Ignore::None);
        let in_port = 2;
        let in_port_name = midi_in.port_name(in_port).unwrap();
        let conn_in = midi_in.connect(in_port, "midir-read-input", move |stamp, message, _| {
            if message.len() >= 2 {
                let m = MidiHandler::get_midi_message(message);
                info!("MidiMessage: {:?}", m);
                m2s_sender.send(SynthMessage::Midi(m.clone())).unwrap();
                let command = message[0] & 0xF0;
                if command == 0xB0 || command == 0xC0 {
                    m2u_sender.send(UiMessage::Midi(m)).unwrap();
                }
            } else {
                info!("Got MIDI message with len {}", message.len());
            }
        }, ()).unwrap();
        conn_in
    }

    pub fn get_midi_message(message: &[u8]) -> MidiMessage {
        let mtype = message[0] & 0xF0;
        let channel = message[0] & 0x0F;
        let param = message[1];
        let mut value = 0;
        if message.len() > 2 {
            value = message[2];
        }
        match message[0] & 0xF0 {
            0x90 => MidiMessage::NoteOn{channel: channel, key: param, velocity: value},
            0x80 => MidiMessage::NoteOff{channel: channel, key: param, velocity: value},
            0xA0 => MidiMessage::KeyAT{channel: channel, key: param, pressure: value},
            0xB0 => MidiMessage::ControlChg{channel: channel, controller: param, value: value},
            0xC0 => MidiMessage::ProgramChg{channel: channel, program: param},
            0xD0 => MidiMessage::ChannelAT{channel: channel, pressure: param},
            0xE0 => {
                let mut pitch: i16 = (param as i16) << 7;
                pitch |= value as i16;
                pitch -= 0x2000;
                MidiMessage::PitchWheel{channel, pitch}
            },
            _ => panic!(),
        }
    }
}
