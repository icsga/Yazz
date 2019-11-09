use std::rc::Rc;
use std::cell::RefCell;

use termion::{color, cursor};
use termion::color::{Black, White, Rgb};

use super::Index;
use super::Observer;
use super::{Value, get_int, get_float};
use super::Widget;

type DialRef = Rc<RefCell<Dial>>;

pub struct Dial {
    pos_x: Index,
    pos_y: Index,
    width: Index,
    height: Index,
    min: Value,
    max: Value,
    value: Value,
    dirty: bool,
}

impl Dial {
    pub fn new(pos_x: Index, pos_y: Index, min: Value, max: Value, value: Value) -> DialRef {
        let width = 2;
        let height = 2;
        let dirty = false;
        Rc::new(RefCell::new(Dial{pos_x, pos_y, width, height, min, max, value, dirty}))
    }

    fn get_index(&self) -> usize {
        match self.value {
            Value::Int(v) => {
                let min = get_int(&self.min);
                let max = get_int(&self.max);
                let range = max - min;
                let offset = min * -1;
                let value = v + offset;
                let index = range / value;
                index as usize
            }
            Value::Float(v) => {
                let min = get_float(&self.min);
                let max = get_float(&self.max);
                let range = max - min;
                let offset = min * -1.0;
                let value = v + offset;
                let index = range / value;
                index as usize
            }
            Value::Str(_) => panic!(),
        }
    }

}

impl Widget for Dial {
    /** Set the dial's position.
     *
     * TODO: Check that new position is valid
     */
    fn set_position(&mut self, x: Index, y: Index) -> bool {
        self.pos_x = x;
        self.pos_y = y;
        true
    }

    /** Set dial's width.
     *
     * TODO: Check that width is valid
     */
    fn set_width(&mut self, width: Index) -> bool {
        self.width = width;
        true
    }

    /** Set dial's height.
     *
     * TODO: Check that height is valid
     */
    fn set_height(&mut self, height: Index) -> bool {
        self.height = height;
        true
    }

    fn set_dirty(&mut self, is_dirty: bool) {
        self.dirty = is_dirty;
    }

    fn is_dirty(&self) -> bool {
        self.dirty
    }

    fn get_position(&self) -> (Index, Index) {
        (self.pos_x, self.pos_y)
    }

    fn get_size(&self) -> (Index, Index) {
        (self.width, self.height)
    }

    fn draw(&self) {
        let index = self.get_index();
        // TODO: Optimize by using array
        let chars = match index {
            0 => "  ",
            1 => "  ",
            2 => "▁ ",
            3 => "\\ ",
            4 => " ▏",
            5 => " /",
            6 => " ▁",
            7 => "  ",
            8 => "  ",
            _ => "  ",
        };
        print!("{}{}{}{}", cursor::Goto(self.pos_x, self.pos_y), color::Bg(White), color::Fg(Black), chars);
        let chars = match index {
            0 => "/ ",
            1 => "▔ ",
            2 => "  ",
            3 => "  ",
            4 => "  ",
            5 => "  ",
            6 => "  ",
            7 => " ▔",
            8 => " \\",
            _ => " ▏",
        };
        print!("{}{}{}", cursor::Goto(self.pos_x, self.pos_y + 1), chars, color::Bg(Rgb(255, 255, 255)));
    }
}

impl Observer for Dial {
    fn update(&mut self, value: Value) {
        self.value = value;
    }
}
