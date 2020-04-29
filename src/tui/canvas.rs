use std::cell::RefCell;
use std::hash::Hash;
use std::rc::Rc;
use std::vec::Vec;

use termion::{color, cursor};

use super::Float;
use super::{Index, Widget, WidgetProperties};

use log::{info, trace, warn};

pub type CanvasRef<Key> = Rc<RefCell<Canvas<Key>>>;

pub struct Canvas<Key: Copy + Eq + Hash> {
    props: WidgetProperties<Key>,
    byte: Vec<char>,
}

impl<Key: Copy + Eq + Hash> Canvas<Key> {
    pub fn new(width: Index, height: Index) -> CanvasRef<Key> {
        let byte = vec!{' '; (width * height) as usize};
        let props = WidgetProperties::new(width, height);
        Rc::new(RefCell::new(Canvas{props, byte}))
    }

    pub fn clear(&mut self) {
        self.byte.iter_mut().map(|x| *x = ' ').count();
    }

    // Set the value of a single cell in the canvas
    pub fn set(&mut self, mut x: Index, mut y: Index, val: char) {
        if x >= self.props.width {
            x = self.props.width - 1;
        }
        if y >= self.props.height {
            y = self.props.height - 1;
        }
        self.byte[(y * self.props.width + x) as usize] = val;
    }

    // Transfer the graph into the output buffer.
    pub fn plot(&mut self, buff: &Vec<Float>, min: Float, max: Float) {
        let scale_y = self.props.height as Float / (max - min);
        let scale_x = self.props.width as Float / buff.len() as Float;
        let offset = min * -1.0;
        if min < 0.0 && max > 0.0 {
            // Calculate position of X axis and print it
            let x_axis = (self.props.height as Float / (max - min)) * (min * -1.0);
            self.plot_x_axis(x_axis as Index);
        }
        // Plot points
        let mut prev_y_pos = self.val_to_y(buff[0], offset, scale_y);
        let mut prev_x_pos: Index = 0;
        let mut y_accu: Float = 0.0;
        let mut y_num_values = 0.0;
        for (i, value) in buff.iter().enumerate() {
            let x_pos = (i as Float * scale_x) as Index;
            y_accu += *value;
            y_num_values += 1.0;
            if x_pos == prev_x_pos && prev_x_pos > 0 {
                // Accumulate values
                continue;
            } else {
                // Next index, build average for previous values
                let mean = y_accu / y_num_values;
                y_accu = 0.0;
                y_num_values = 0.0;
                let y_pos = self.val_to_y(mean, offset, scale_y);


                let diff: i64 = Self::diff(y_pos, prev_y_pos);
                if diff > 1 {
                    // Current and previous values are more than one row apart, fill the space between
                    let end_y_pos: Index;
                    let start_y_pos: Index;
                    if y_pos > prev_y_pos {
                        start_y_pos = prev_y_pos + 1;
                        end_y_pos = y_pos;
                    } else {
                        start_y_pos = prev_y_pos;
                        end_y_pos = y_pos + 1;
                    };
                    let (x1, from, x2, to) = Self::sort(x_pos as Index, start_y_pos, x_pos as Index, end_y_pos);
                    for i in from..to {
                        self.set(x1, i, '⋅');
                    }

                }

                // Stretch value over skipped places if needed
                if x_pos - prev_x_pos > 1 {
                    for x in prev_x_pos..x_pos {
                        self.set(x as Index, y_pos, '∘');
                    }
                } else {
                    self.set(x_pos, y_pos, '∘');
                }
                prev_x_pos = x_pos;
                prev_y_pos = y_pos;
            }
        }
        self.props.set_dirty(true);
    }

    // Transform a value to a graph coordinate
    fn val_to_y(&self, value: Float, offset: Float, scale: Float) -> Index {
        ((value + offset) * scale) as Index
    }

    // Draw the x-axis line into the buffer
    fn plot_x_axis(&mut self, y_pos: Index) {
        for x in 0..self.props.width {
            self.set(x, y_pos, '-');
        }
    }

    fn diff(a: Index, b: Index) -> i64 {
        let result = if a > b { a - b } else { b - a };
        result as i64
    }

    fn sort(x_a: Index, y_a: Index, x_b: Index, y_b: Index) -> (Index, Index, Index, Index) {
        if y_a < y_b { (x_a, y_a, x_b, y_b) } else { (x_b, y_b, x_a, y_a) }
    }
}

impl<Key: Copy + Eq + Hash> Widget<Key> for Canvas<Key> {
    fn get_widget_properties_mut<'a>(&'a mut self) -> &'a mut WidgetProperties<Key> {
        return &mut self.props;
    }

    fn get_widget_properties<'a>(&'a self) -> &'a WidgetProperties<Key> {
        return &self.props;
    }

    // Print the output buffer to the screen
    fn draw(&self) {
        let pos_x = self.props.pos_x;
        let pos_y = self.props.pos_y;
        print!("{}{}{}", cursor::Goto(self.props.pos_x, self.props.pos_y), color::Bg(self.props.colors.bg_dark), color::Fg(self.props.colors.fg_light2));
        for y in 0..self.props.height {
            for x in 0..self.props.width {
                print!("{}{}", cursor::Goto(x + pos_x, pos_y + ((self.props.height - 1) - y)), self.byte[(y * self.props.width + x) as usize]);
            }
        }
    }
}
