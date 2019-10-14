extern crate term_cursor as cursor;

use std::vec::Vec;

pub struct Canvas {
    x_size: usize,
    y_size: usize,
    byte: Vec<char>,
}

impl Canvas {
    pub fn new(x_size: usize, y_size: usize) -> Canvas {
        let byte = vec!{' '; x_size * y_size};
        Canvas{x_size, y_size, byte}
    }

    pub fn clear(&mut self) {
        self.byte.iter_mut().map(|x| *x = ' ').count();
        let y = ((self.y_size / 2) - 1) * self.x_size;
        for x in 0..self.x_size {
            self.byte[y + x] = '-';
        }
    }

    pub fn set(&mut self, mut x: usize, mut y: usize, val: char) {
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
                print!("{}{}", cursor::Goto((x + x_pos) as i32, (y_pos + (self.y_size - y)) as i32), self.byte[y * self.x_size + x]);
            }
        }
    }
}
