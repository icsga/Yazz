use std::cell::RefCell;
use std::rc::Rc;
use std::vec::Vec;

use termion::{color, cursor};

use super::Float;
use super::{Index, Widget, WidgetProperties};

use log::{info, trace, warn};

type CanvasRef = Rc<RefCell<Canvas>>;

pub struct Canvas {
    props: WidgetProperties,
    byte: Vec<char>,
}

impl Canvas {
    pub fn new(width: Index, height: Index) -> CanvasRef {
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
            if x_pos == prev_x_pos {
                // Accumulate values
                continue;
            } else {
                // Next index, build average for previous values
                let mean = y_accu / y_num_values;
                y_accu = 0.0;
                y_num_values = 0.0;
                let y_pos = self.val_to_y(mean, offset, scale_y);


                /*
                let diff: i64 = Canvas::diff(y_pos, prev_y_pos);
                if diff > 1 {
                    // Current and previous values are more than one row apart, fill the space between
                    let (x1, from, x2, to) = Canvas::sort(x_pos as Index - 1, prev_y_pos, x_pos as Index, y_pos);
                    let halfpoint = from + (diff / 2) as Index;
                    for i in from..halfpoint {
                        self.set(x1, i, '|');
                    }
                    for i in halfpoint..to {
                        self.set(x2, i, '|');
                    }

                }
                */

                // Stretch value over skipped places if needed
                for x in prev_x_pos..x_pos {
                    self.set(x as Index, y_pos, '∘');
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

impl Widget for Canvas {
    fn get_widget_properties_mut<'a>(&'a mut self) -> &'a mut WidgetProperties {
        return &mut self.props;
    }

    fn get_widget_properties<'a>(&'a self) -> &'a WidgetProperties {
        return &self.props;
    }

    // Print the output buffer to the screen
    fn draw(&self) {
        let pos_x = self.props.pos_x;
        let pos_y = self.props.pos_y;
        print!("{}{}{}", cursor::Goto(self.props.pos_x, self.props.pos_y), color::Bg(self.props.colors.bg_dark), color::Fg(self.props.colors.fg_light2));
        for y in 0..self.props.height {
            for x in 0..self.props.width {
                print!("{}{}", cursor::Goto(x + pos_x, pos_y + (self.props.height - y)), self.byte[(y * self.props.width + x) as usize]);
            }
        }
    }
}
