extern crate term_cursor as cursor;

use std::vec::Vec;

pub struct Canvas {
    x_size: usize,
    y_size: usize,
    byte: Vec<u8>,
}

impl Canvas {
    pub fn new(x_size: usize, y_size: usize) -> Canvas {
        let byte = vec!{' ' as u8; x_size * y_size};
        Canvas{x_size, y_size, byte}
    }

    pub fn clear(&mut self) {
        self.byte.iter_mut().map(|x| *x = ' ' as u8).count();
    }

    pub fn set(&mut self, mut x: usize, mut y: usize, val: u8) {
        if x >= self.x_size {
            x = self.x_size - 1;
        }
        if y >= self.y_size {
            y = self.y_size - 1;
        }
        self.byte[y * self.x_size + x] = val;
    }

    pub fn render(&self, x_pos: usize, y_pos: usize) {
        for y in 0..self.y_size {
            for x in 0..self.x_size {
                print!("{}{}", cursor::Goto((x + x_pos) as i32, (y_pos + (self.y_size - y)) as i32), self.byte[y * self.x_size + x] as char);
            }
        }
    }
}
