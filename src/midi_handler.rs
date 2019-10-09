use midir::{MidiInput, MidiInputConnection, Ignore};

use crossbeam_channel::{Sender, Receiver};

pub struct MidiHandler {
    m2s_sender: Sender<MidiMessage>,
}

pub struct MidiMessage {
    pub mtype: u8,
    pub param: u8,
    pub value: u8
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
}
