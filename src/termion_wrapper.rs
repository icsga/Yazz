extern crate termion;

use termion::clear;
use termion::cursor;
use termion::cursor::{DetectCursorPos, Goto};
use termion::event::Key;
use termion::input::TermRead;
use termion::raw::{IntoRawMode, RawTerminal};

use super::SynthParam;
use super::Tui;

use std::io::{Write, stdout, stdin};
use std::thread::spawn;

pub struct TermionWrapper {
    stdout: RawTerminal<std::io::Stdout>,
    //stdout: std::io::Stdout,
    tui: Tui,
}

impl TermionWrapper {
    pub fn new(tui: Tui) -> TermionWrapper {
        TermionWrapper{
            stdout: stdout().into_raw_mode().unwrap(),
            //stdout: stdout(),
            tui: tui
        }
    }

    pub fn run(mut termion: TermionWrapper) -> std::thread::JoinHandle<()> {
        let handler = spawn(move || {
            let mut exit = false;
            let stdin = stdin();

            for c in stdin.keys() {
                let c = c.unwrap();
                match c {
                    // Exit.
                    Key::Char('q') => { exit = true; break},
                    _              => termion.tui.handle_input(c),
                }
                termion.stdout.flush().unwrap();
            }
            if exit {
                println!("");
                return;
            }
        });
        handler
    }

    pub fn cursor_pos(&mut self) -> (u16, u16) {
        self.stdout.cursor_pos().unwrap()
    }
}

