
use termion::cursor;
use termion::event::*;
use termion::input::{TermRead, MouseTerminal};
use termion::raw::{IntoRawMode, RawTerminal};

use super::UiMessage;

use crossbeam_channel::{Sender};
use log::info;

use std::io::{Write, stdout, stdin};
use std::thread::spawn;

pub struct TermionWrapper {
    stdout: MouseTerminal<RawTerminal<std::io::Stdout>>,
}

impl TermionWrapper {
    pub fn new() -> Result<TermionWrapper, std::io::Error> {
        println!("{}", cursor::Hide);
        let t = TermionWrapper{
            stdout: MouseTerminal::from(stdout().into_raw_mode()?)
        };
        Ok(t)
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
                            Key::F(12) => {
                                info!("Stopping terminal handler");
                                exit = true;
                                to_ui_sender.send(UiMessage::Exit).unwrap();
                                break;
                            }
                            _ => to_ui_sender.send(UiMessage::Key(c)).unwrap(),
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
                println!("{}", termion::cursor::Show);
                return;
            }
        });
        handler
    }
}

