
use termion::clear;
use termion::cursor;
use termion::cursor::{DetectCursorPos, Goto, Hide};
use termion::event::Key;
use termion::input::TermRead;
use termion::raw::{IntoRawMode, RawTerminal};

use super::SynthParam;
use super::{UiMessage, SynthMessage};

use crossbeam_channel::{Sender};

use std::io::{Write, stdout, stdin};
use std::thread::spawn;

pub struct TermionWrapper {
    stdout: RawTerminal<std::io::Stdout>,
}

impl TermionWrapper {
    pub fn new() -> TermionWrapper {
        println!("{}", cursor::Hide);
        TermionWrapper{
            stdout: stdout().into_raw_mode().unwrap()
        }
    }

    pub fn run(mut termion: TermionWrapper, to_ui_sender: Sender<UiMessage>) -> std::thread::JoinHandle<()> {
        let handler = spawn(move || {
            let mut exit = false;
            let stdin = stdin();

            for c in stdin.keys() {
                let c = c.unwrap();
                match c {
                    // Exit.
                    Key::Char('q') => { exit = true; break},
                    _              => to_ui_sender.send(UiMessage::Key(c)).unwrap(),
                };
                termion.stdout.flush().unwrap();
            }
            if exit {
                println!("");
                return;
            }
        });
        handler
    }
}

