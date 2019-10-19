extern crate term_cursor as cursor;

use std::vec::Vec;

use log::{info, trace, warn};

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

    pub fn plot(&mut self, buff: &Vec<f32>, min: f32, max: f32) {
        let scale = self.y_size as f32 / (max - min);
        let offset = min * -1.0;
        info!("Plotting buff, min={}, max={}, scale={}, offset={}",
              min, max, scale, offset);
        if min < 0.0 && max > 0.0 {
            // Calculate position of X axis and print it
            let x_axis = (self.y_size as f32 / (max - min)) * (min * -1.0);
            self.plot_x_axis(x_axis as usize);
        }
        // Plot points
        let mut prev = self.val_to_y(buff[0], offset, scale);
        for (x_pos, value) in buff.iter().enumerate() {
            let y_pos = self.val_to_y(*value, offset, scale);

            let diff: i64 = Canvas::abs((y_pos - prev) as i64);
            if diff > 1 {
                // Current and previous values are more than one row apart, fill the space between
                let (x1, from, x2, to) = Canvas::sort(x_pos - 1, prev, x_pos, y_pos);
                let halfpoint = from + (diff / 2) as usize;
                for i in from..halfpoint {
                    self.set(x1, i, '|');
                }
                for i in halfpoint..to {
                    self.set(x2, i, '|');
                }

            }
            self.set(x_pos, y_pos, '∘');
            prev = y_pos;
        }
    }

    pub fn render(&self, x_pos: usize, y_pos: usize) {
        for y in 0..self.y_size {
            for x in 0..self.x_size {
                print!("{}{}", cursor::Goto((x + x_pos) as i32, (y_pos + (self.y_size - y)) as i32), self.byte[y * self.x_size + x]);
            }
        }
    }

    fn val_to_y(&self, value: f32, offset: f32, scale: f32) -> usize {
        ((value + offset) * scale) as usize
    }

    fn plot_x_axis(&mut self, y_pos: usize) {
        info!("Plotting x-axis at {}", y_pos);
        for x in 0..self.x_size {
            self.set(x, y_pos, '-');
        }
    }

    fn abs(value: i64) -> i64 {
        if value < 0 { value * -1 } else { value }
    }

    fn sort(x_a: usize, y_a: usize, x_b: usize, y_b: usize) -> (usize, usize, usize, usize) {
        if y_a < y_b { (x_a, y_a, x_b, y_b) } else { (x_b, y_b, x_a, y_a) }
    }
}
