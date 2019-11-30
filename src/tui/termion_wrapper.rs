
use termion::clear;
use termion::cursor;
use termion::cursor::{DetectCursorPos, Goto, Hide};
use termion::event::*;
use termion::input::{TermRead, MouseTerminal};
use termion::raw::{IntoRawMode, RawTerminal};

use super::SynthParam;
use super::{UiMessage, SynthMessage};

use crossbeam_channel::{Sender};

use std::io::{Write, stdout, stdin};
use std::thread::spawn;

pub struct TermionWrapper {
    stdout: MouseTerminal<RawTerminal<std::io::Stdout>>,
}

impl TermionWrapper {
    pub fn new() -> TermionWrapper {
        println!("{}", cursor::Hide);
        TermionWrapper{
            stdout: MouseTerminal::from(stdout().into_raw_mode().unwrap())
        }
    }

    pub fn run(mut termion: TermionWrapper, to_ui_sender: Sender<UiMessage>) -> std::thread::JoinHandle<()> {
        let handler = spawn(move || {
            let mut exit = false;
            let stdin = stdin();

            for e in stdin.events() {
                let e = e.unwrap();
                match e {
                    Event::Key(c) => {
                        match c {
                            // Exit.
                            Key::Char('q') => { exit = true; break},
                            _              => to_ui_sender.send(UiMessage::Key(c)).unwrap(),
                        };
                        termion.stdout.flush().unwrap();
                    }
                    Event::Mouse(m) => {
                        match m {
                            MouseEvent::Press(_, x, y) => to_ui_sender.send(UiMessage::MousePress{x, y}).unwrap(),
                            MouseEvent::Hold(x, y) => to_ui_sender.send(UiMessage::MouseHold{x, y}).unwrap(),
                            MouseEvent::Release(x, y) => to_ui_sender.send(UiMessage::MouseRelease{x, y}).unwrap(),
                        }
                    }
                    _ => {}
                }
            }
            if exit {
                println!("");
                return;
            }
        });
        handler
    }
}

