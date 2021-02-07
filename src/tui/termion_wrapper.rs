
use termion::cursor;
use termion::event::*;
use termion::input::{TermRead, MouseTerminal};
use termion::raw::{IntoRawMode, RawTerminal};
use termion::color::DetectColors;

use super::UiMessage;
use super::Index;

use crossbeam_channel::{Sender};
use log::info;

use std::io::{stdout, stdin};
use std::thread::spawn;

pub struct TermionWrapper {
    stdout: MouseTerminal<RawTerminal<std::io::Stdout>>,
}

impl TermionWrapper {
    pub fn new() -> Result<TermionWrapper, std::io::Error> {
        println!("{}", cursor::Hide);
        let mut t = TermionWrapper{
            stdout: MouseTerminal::from(stdout().into_raw_mode()?),
        };
        let count = t.stdout.available_colors().unwrap();
        println!("Available colors: {}", count);
        Ok(t)
    }

    pub fn run(to_ui_sender: Sender<UiMessage>) -> std::thread::JoinHandle<()> {
        spawn(move || {
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
                        //termion.stdout.flush().unwrap();
                    }
                    Event::Mouse(m) => {
                        match m {
                            MouseEvent::Press(_, x, y) => to_ui_sender.send(UiMessage::MousePress{x: x as Index, y: y as Index}).unwrap(),
                            MouseEvent::Hold(x, y) => to_ui_sender.send(UiMessage::MouseHold{x: x as Index, y: y as Index}).unwrap(),
                            MouseEvent::Release(x, y) => to_ui_sender.send(UiMessage::MouseRelease{x: x as Index, y: y as Index}).unwrap(),
                        }
                    }
                    _ => {}
                }
            }
            if exit {
                println!("{}", termion::cursor::Show);
                return;
            }
        })
    }
}

