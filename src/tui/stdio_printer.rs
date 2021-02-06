use termion::{color, cursor};
use termion::color::AnsiValue;

use super::printer::{Index, Printer};

use std::io::{stdout, Write};

pub struct StdioPrinter {
    last_fg: AnsiValue,
    last_bg: AnsiValue,
    last_x: Index,
    last_y: Index
}

impl StdioPrinter {
    pub fn new() -> Self {
        StdioPrinter{
            last_fg: AnsiValue(15),
            last_bg: AnsiValue(0),
            last_x: 0,
            last_y: 0
        }
    }
}

impl Printer for StdioPrinter {
    fn set_color(&mut self, fg_color: AnsiValue, bg_color: AnsiValue) {
        //if !matches!(self.last_fg, fg_color) || !matches!(self.last_bg, bg_color) {
            print!("{}{}", color::Bg(bg_color), color::Fg(fg_color));
            //self.last_bg = bg_color;
            //self.last_fg = fg_color;
        //}
    }

    // Print some text
    fn print(&mut self, x: Index, y: Index, text: &str) {
        // TODO: Use string to buffer adjacent x-values
        if y == self.last_y && x == self.last_x {
            // No need to move cursor, alredy at right position
            print!("{}", text);
            self.last_x = self.last_x + text.len();
        } else {
            print!("{}{}", cursor::Goto(x as u16, y as u16), text);
            self.last_x = x + text.len();
            self.last_y = y;
        }
    }

    // Update the screen contents
    fn update(&mut self) {
        stdout().flush().ok();
    }
}
