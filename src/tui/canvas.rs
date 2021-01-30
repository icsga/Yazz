use std::cell::RefCell;
use std::hash::Hash;
use std::rc::Rc;
use std::vec::Vec;

use super::Float;
use super::{Index, Printer, Widget, WidgetProperties};

pub type CanvasRef<Key> = Rc<RefCell<Canvas<Key>>>;

pub struct Canvas<Key: Copy + Eq + Hash> {
    props: WidgetProperties<Key>,
    byte: Vec<char>,
    line_buff: RefCell<String>,
}

impl<Key: Copy + Eq + Hash> Canvas<Key> {
    pub fn new(width: Index, height: Index) -> CanvasRef<Key> {
        let byte = vec!{' '; (width * height) as usize};
        let props = WidgetProperties::new(width, height);
        let line_buff = RefCell::new(String::with_capacity(width));
        Rc::new(RefCell::new(Canvas{props, byte, line_buff}))
    }

    pub fn clear(&mut self) {
        self.byte.iter_mut().for_each(|x| *x = ' ');
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

    /** Transfer the graph into the output buffer.
     *
     * The number of values to print can be greater or less than the width of
     * the Canvas, so we need to either combine or stretch the values to make
     * them fit.
     *
     * To make things look nice, we also fill spaces between points that are in
     * neighboring columns, but more than one row apart.
     */
    pub fn plot(&mut self, buff: &[Float], min: Float, max: Float) {
        let (scale_x, scale_y, offset) = self.calc_scaling(min, max, buff.len());
        if min < 0.0 && max > 0.0 {
            // X axis lies on screen. Calculate its position and print it.
            let x_axis = self.calc_x_axis_position(min, max);
            self.plot_x_axis(x_axis as Index);
        }

        // Plot points
        let mut x_pos: Index = 0;
        let mut y_pos: Index;
        let mut prev_y_pos = self.val_to_y(buff[0], offset, scale_y, min, max);
        let mut prev_x_pos: Index = 0;
        let mut y_accu: Float = 0.0;

        for (i, value) in buff.iter().enumerate() {
            x_pos = (i as Float * scale_x) as Index;
            if x_pos == prev_x_pos {
                // Take min/ max of all values that fall on the same column on screen
                if *value >= 0.0 && *value > y_accu
                || *value < 0.0 && *value < y_accu {
                    y_accu = *value;
                }
            } else {
                // Advanced to next column, print previous max value
                y_pos = self.val_to_y(y_accu, offset, scale_y, min, max);
                self.draw_point(prev_x_pos, x_pos, prev_y_pos, y_pos);
                y_accu = *value;
                prev_x_pos = x_pos;
                prev_y_pos = y_pos;
            }
        }

        // Plot last point
        y_pos = self.val_to_y(y_accu, offset, scale_y, min, max);
        self.draw_point(prev_x_pos, x_pos, prev_y_pos, y_pos);
        self.props.set_dirty(true);
    }

    fn draw_point(&mut self, prev_x_pos: Index, x_pos: Index, prev_y_pos: Index, y_pos: Index) {
        let diff: i64 = Self::diff(y_pos, prev_y_pos);
        if diff > 1 {
            // Current and previous points are more than one row apart, fill
            // the space between.
            self.connect_points(prev_x_pos, y_pos, prev_y_pos);
        }

        // Draw actualy point
        self.set(prev_x_pos, y_pos, '∘');

        // Stretch point over skipped columns if needed
        if x_pos - prev_x_pos > 1 {
            self.stretch_point(prev_x_pos + 1, x_pos, y_pos);
        }
    }

    // Draw a vertical line between values more than one row apart.
    fn connect_points(&mut self, x_pos: Index, y_pos: Index, prev_y_pos: Index) {
        let end_y_pos: Index;
        let start_y_pos: Index;
        if y_pos > prev_y_pos {
            start_y_pos = prev_y_pos + 1;
            end_y_pos = y_pos;
        } else {
            start_y_pos = prev_y_pos;
            end_y_pos = y_pos + 1;
        };
        let (x1, from, _x2, to) = Self::sort(x_pos as Index, start_y_pos, x_pos as Index, end_y_pos);
        for i in from..to {
            self.set(x1, i, '⋅');
        }
    }

    fn stretch_point(&mut self, prev_x_pos: Index, x_pos: Index, y_pos: Index) {
        for x in prev_x_pos..x_pos {
            self.set(x as Index, y_pos, '∘');
        }
    }

    pub fn calc_scaling(&self, min: Float, max: Float, num_values: usize) -> (Float, Float, Float) {
        let scale_x = self.props.width as Float / num_values as Float;
        let scale_y = (self.props.height - 1) as Float / (max - min);
        let offset = min * -1.0;
        (scale_x, scale_y, offset)
    }

    pub fn calc_x_axis_position(&self, min: Float, max: Float) -> Index {
        ((self.props.height as Float / (max - min)) * (min * -1.0)) as Index
    }

    // Transform a value to a graph coordinate
    fn val_to_y(&self, mut value: Float, offset: Float, scale: Float, min: Float, max: Float) -> Index {
        if value < min {
            value = min;
        } else if value > max {
            value = max;
        }
        ((value + offset) * scale).round() as Index
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
    fn get_widget_properties_mut(&mut self) -> &mut WidgetProperties<Key> {
        &mut self.props
    }

    fn get_widget_properties(&self) -> &WidgetProperties<Key> {
        &self.props
    }

    // Print the output buffer to the screen
    fn draw(&self, p: &mut dyn Printer) {
        let pos_x = self.props.pos_x;
        let pos_y = self.props.pos_y;
        let mut buff = self.line_buff.borrow_mut();
        p.set_color(self.props.colors.fg_base, self.props.colors.bg_base_l);
        for y in 0..self.props.height {
            buff.clear();
            for x in 0..self.props.width {
                (*buff).push(self.byte[(y * self.props.width + x) as usize]);
            }
            let y_coord = pos_y + ((self.props.height - 1) - y);
            p.print(pos_x, y_coord, &buff);
        }
    }
}

// ----------
// Unit tests
// ----------

#[test]
fn scaling_works() {
    let canvas: CanvasRef<u32> = Canvas::new(100, 21); // 100 wide, 21 high
    let c = canvas.borrow();

    let (scale_x, scale_y, offset) = c.calc_scaling(-1.0, 1.0, 100); // 100 values from -1.0 to 1.0
    assert_eq!(scale_x, 1.0);
    assert_eq!(scale_y, 10.0); // 10.5 up and 10.5 down
    assert_eq!(offset, 1.0);

    let (scale_x, scale_y, offset) = c.calc_scaling(0.0, 1.0, 200); // 200 values from 0.0 to 1.0
    assert_eq!(scale_x, 0.5);
    assert_eq!(scale_y, 20.0); // 21 up
    assert_eq!(offset, 0.0);
}

#[test]
fn x_axis_is_placed_correctly() {
    let canvas: CanvasRef<u32> = Canvas::new(100, 21);
    let c = canvas.borrow();
    let x_axis = c.calc_x_axis_position(-1.0, 1.0);
    assert_eq!(x_axis, 10);
}

#[test]
fn translation_works() {
    let canvas: CanvasRef<u32> = Canvas::new(100, 21);
    let c = canvas.borrow();

    let (min, max) = (-1.0, 1.0);
    let (_, scale_y, offset) = c.calc_scaling(min, max, 100);
    let y = c.val_to_y(0.0, offset, scale_y, min, max);
    assert_eq!(y, 10);
    let y = c.val_to_y(-1.0, offset, scale_y, min, max);
    assert_eq!(y, 0);
    let y = c.val_to_y(1.0, offset, scale_y, min, max);
    assert_eq!(y, 20);

    let (min, max) = (0.0, 1.0);
    let (_, scale_y, offset) = c.calc_scaling(min, max, 100);
    let y = c.val_to_y(0.0, offset, scale_y, min, max);
    assert_eq!(y, 0);
    let y = c.val_to_y(0.5, offset, scale_y, min, max);
    assert_eq!(y, 10);
    let y = c.val_to_y(1.0, offset, scale_y, min, max);
    assert_eq!(y, 20);
}

#[test]
fn out_of_range_values_are_caught() {
    let canvas: CanvasRef<u32> = Canvas::new(100, 21);
    let c = canvas.borrow();

    let (min, max) = (-1.0, 1.0);
    let (_, scale_y, offset) = c.calc_scaling(min, max, 100);
    let y = c.val_to_y(-1.1, offset, scale_y, min, max);
    assert_eq!(y, 0);
    let y = c.val_to_y(1.1, offset, scale_y, min, max);
    assert_eq!(y, 20);

    let (min, max) = (0.0, 1.0);
    let (_, scale_y, offset) = c.calc_scaling(min, max, 100);
    let y = c.val_to_y(-0.1, offset, scale_y, min, max);
    assert_eq!(y, 0);
    let y = c.val_to_y(1.1, offset, scale_y, min, max);
    assert_eq!(y, 20);
}
