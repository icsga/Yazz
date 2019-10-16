use midir::{MidiInput, MidiInputConnection, Ignore};

use crossbeam_channel::{Sender, Receiver};

use super::{SynthMessage, UiMessage};

pub struct MidiHandler {
    m2s_sender: Sender<MidiMessage>,
}

pub struct MidiMessage {
    pub mtype: u8,
    pub param: u8,
    pub value: u8
}

impl MidiMessage {
    pub fn get_message_type(&self) -> MessageType {
        match self.mtype & 0xF0 {
            0x80 => MessageType::NoteOff,
            0x90 => MessageType::NoteOn,
            0xA0 => MessageType::KeyAT,
            0xB0 => MessageType::ControlChg,
            0xC0 => MessageType::ProgramChg,
            0xD0 => MessageType::ChannelAT,
            0xE0 => MessageType::PitchWheel,
            _ => {
                println!("\r\nGot MidiMessage {}\n\r", self.mtype);
                panic!();
            }
        }
    }

    pub fn get_channel(&self) -> u8 {
        self.mtype & 0x0F
    }

    pub fn get_value(&self) -> u64 {
        match self.get_message_type() {
            MessageType::NoteOn |
            MessageType::NoteOff |
            MessageType::KeyAT |
            MessageType::ControlChg |
            MessageType::ProgramChg |
            MessageType::ChannelAT => self.value as u64,
            MessageType::PitchWheel => ((self.value << 7) + self.param) as u64,
        }
    }
}

pub enum MessageType {
    NoteOn = 0x08,
    NoteOff = 0x90,
    KeyAT = 0xA0,
    ControlChg = 0xB0,
    ProgramChg = 0xC0,
    ChannelAT = 0xD0,
    PitchWheel = 0xE0
}

impl MidiHandler {
    pub fn new(m2s_sender: Sender<MidiMessage>) -> MidiHandler {
        MidiHandler{m2s_sender}
    }

    pub fn run(m2s_sender: Sender<SynthMessage>, m2u_sender: Sender<UiMessage>) -> MidiInputConnection<()> {
        let input = String::new();
        let mut midi_in = MidiInput::new("midir reading input").unwrap();
        midi_in.ignore(Ignore::None);
        let in_port = 1;
        let in_port_name = midi_in.port_name(in_port).unwrap();
        let conn_in = midi_in.connect(in_port, "midir-read-input", move |stamp, message, _| {
            if message.len() == 3 {
                let m = MidiMessage{mtype: message[0], param: message[1], value: message[2]};
                m2s_sender.send(SynthMessage::Midi(m)).unwrap();
                if message[0] & 0xF0 == 0xB0 {
                    let m = MidiMessage{mtype: message[0], param: message[1], value: message[2]};
                    m2u_sender.send(UiMessage::Midi(m)).unwrap();
                }
            }
        }, ()).unwrap();
        conn_in
    }
}
