use midir::{MidiInput, Ignore};

use std::sync::mpsc::{Sender, Receiver};

pub struct MidiHandler {
    keymap: [f32; 127],
    m2s_sender: Sender<MidiMessage>
}

pub struct MidiMessage {
    pub mtype: u8,
    pub param: u8,
    pub value: u8
}

impl MidiHandler {
    pub fn new(m2s_sender: Sender<MidiMessage>) -> MidiHandler {
        let mut keymap: [f32; 127] = [0.0; 127];
        MidiHandler::calculate_keymap(&mut keymap, 440.0);
        MidiHandler{
            keymap,
            m2s_sender
        }
    }

    pub fn run(&self, port: MidiInput) {
        let in_port = 2;
        let _conn_in = port.connect(2, "midir-read-input", move |stamp, message, _| {
            //println!("{}: {:?} (len = {})", stamp, message, message.len());
            if message.len() == 3 {
                let m = MidiMessage{mtype: message[0], param: message[1], value: message[2]};
                //self.m2s_sender.send(m).unwrap();
            }
        }, ()).unwrap();
    }

    fn calculate_keymap(map: &mut[f32; 127], reference_pitch: f32) {
        for i in 0..127 {
           map[i] = (reference_pitch / 32.0) * (2.0f32.powf((i as f32 - 9.0) / 12.0));
        }
    }
}
