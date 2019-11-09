use std::cell::RefCell;
use std::rc::Rc;

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
    pub fn new(min: Value, max: Value, value: Value) -> DialRef {
        let pos_x: Index = 0;
        let pos_y: Index = 0;
        let width = 2;
        let height = 2;
        let dirty = false;
        Rc::new(RefCell::new(Dial{pos_x, pos_y, width, height, min, max, value, dirty}))
    }

    pub fn get_index(&self, value: &Value) -> usize {
        match value {
            Value::Int(v) => {
                let min = get_int(&self.min);
                let max = get_int(&self.max);
                let offset = min * -1;
                let range = (max - min) as f64;
                let scale = 8.0 / range;
                let value = (v + offset) as f64;
                let index = (value * scale) as usize;
                index
            }
            Value::Float(v) => {
                let min = get_float(&self.min);
                let max = get_float(&self.max);
                let offset = min * -1.0;
                let range = max - min;
                let scale = 8.0 / range;
                let value = v + offset;
                let index = (value * scale) as usize;
                index
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
        let index = self.get_index(&self.value);
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
            _ => "  ",
            //_ => "  ",
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
            _ => " \\",
            //_ => " ▏",
        };
        print!("{}{}{}", cursor::Goto(self.pos_x, self.pos_y + 1), chars, color::Bg(Rgb(255, 255, 255)));
    }
}

impl Observer for Dial {
    fn update(&mut self, value: Value) {
        self.value = value;
        self.set_dirty(true);
    }
}

#[test]
fn test_dial_translation() {
    // =====
    // Float
    // =====
    // Case 1: 0.0 - 1.0
    let d = Dial::new(Value::Float(0.0), Value::Float(1.0), Value::Float(0.0));
    assert_eq!(d.borrow().get_index(&Value::Float(0.0)), 0);
    assert_eq!(d.borrow().get_index(&Value::Float(0.5)), 4);
    assert_eq!(d.borrow().get_index(&Value::Float(1.0)), 8);

    // Case 2: -1.0 - 1.0
    let d = Dial::new(Value::Float(-1.0), Value::Float(1.0), Value::Float(0.0));
    assert_eq!(d.borrow().get_index(&Value::Float(-1.0)), 0);
    assert_eq!(d.borrow().get_index(&Value::Float(0.0)), 4);
    assert_eq!(d.borrow().get_index(&Value::Float(1.0)), 8);

    // Case 3: 2.0 - 10.0
    let d = Dial::new(Value::Float(2.0), Value::Float(10.0), Value::Float(0.0));
    assert_eq!(d.borrow().get_index(&Value::Float(2.0)), 0);
    assert_eq!(d.borrow().get_index(&Value::Float(6.0)), 4);
    assert_eq!(d.borrow().get_index(&Value::Float(10.0)), 8);

    // ===
    // Int
    // ===
    // Case 1: 0 - 8
    let d = Dial::new(Value::Int(0), Value::Int(8), Value::Int(0));
    assert_eq!(d.borrow().get_index(&Value::Int(0)), 0);
    assert_eq!(d.borrow().get_index(&Value::Int(4)), 4);
    assert_eq!(d.borrow().get_index(&Value::Int(8)), 8);

    // Case 2: -4 - 4
    let d = Dial::new(Value::Int(-4), Value::Int(4), Value::Int(0));
    assert_eq!(d.borrow().get_index(&Value::Int(-4)), 0);
    assert_eq!(d.borrow().get_index(&Value::Int(0)), 4);
    assert_eq!(d.borrow().get_index(&Value::Int(4)), 8);

    // Case 3: 2 - 10
    let d = Dial::new(Value::Int(2), Value::Int(10), Value::Int(0));
    assert_eq!(d.borrow().get_index(&Value::Int(2)), 0);
    assert_eq!(d.borrow().get_index(&Value::Int(6)), 4);
    assert_eq!(d.borrow().get_index(&Value::Int(10)), 8);
}
