use std::cell::RefCell;
use std::rc::Rc;

use termion::{color, cursor};

use super::Index;
use super::Observer;
use super::Scheme;
use super::{Value, get_int, get_float};
use super::Widget;

type BarRef = Rc<RefCell<Bar>>;

pub struct Bar {
    pos_x: Index,
    pos_y: Index,
    width: Index,
    height: Index,
    min: Value,
    max: Value,
    value: Value,
    dirty: bool,
    colors: Rc<Scheme>,
}

impl Bar {
    pub fn new(min: Value, max: Value, value: Value) -> BarRef {
        let pos_x: Index = 0;
        let pos_y: Index = 0;
        let width = 10;
        let height = 1;
        let dirty = false;
        let colors = Rc::new(Scheme::new());
        Rc::new(RefCell::new(Bar{pos_x, pos_y, width, height, min, max, value, dirty, colors}))
    }

    fn get_length(&self, value: &Value) -> usize {
        match value {
            Value::Int(v) => {
                let min = get_int(&self.min);
                let max = get_int(&self.max);
                let offset = min * -1;
                let range = (max - min) as f64;
                let scale = self.width as f64 / range;
                let value = (v + offset) as f64;
                let length = (value * scale) as usize;
                length
            }
            Value::Float(v) => {
                let min = get_float(&self.min);
                let max = get_float(&self.max);
                let offset = min * -1.0;
                let range = max - min;
                let scale = self.width as f64 / range;
                let value = v + offset;
                let length = (value * scale) as usize;
                length
            }
            Value::Str(_) => panic!(),
        }
    }
}

impl Widget for Bar {
    /** Set the bar's position.
     *
     * TODO: Check that new position is valid
     */
    fn set_position(&mut self, x: Index, y: Index) -> bool {
        self.pos_x = x;
        self.pos_y = y;
        true
    }

    /** Set bar's width.
     *
     * TODO: Check that width is valid
     */
    fn set_width(&mut self, width: Index) -> bool {
        self.width = width;
        true
    }

    /** Set bar's height.
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

    fn set_color_scheme(&mut self, colors: Rc<Scheme>) {
        self.colors = colors;
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
        let index = self.get_length(&self.value);
        // TODO: Optimize by using array
        print!("{}{}{}", cursor::Goto(self.pos_x, self.pos_y), color::Bg(self.colors.bg_light), color::Fg(self.colors.fg_dark2));
        for i in 0..index {
            print!("‾");
        }
    }
}

impl Observer for Bar {
    fn update(&mut self, value: Value) {
        self.value = value;
        self.set_dirty(true);
    }
}

#[test]
fn test_bar_translation() {
    // =====
    // Float
    // =====
    // Case 1: 0.0 - 1.0
    let b = Bar::new(Value::Float(0.0), Value::Float(1.0), Value::Float(0.0));
    assert_eq!(b.borrow().get_length(&Value::Float(0.0)), 0);
    assert_eq!(b.borrow().get_length(&Value::Float(0.5)), 5);
    assert_eq!(b.borrow().get_length(&Value::Float(1.0)), 10);

    // Case 2: -1.0 - 1.0
    let b = Bar::new(Value::Float(-1.0), Value::Float(1.0), Value::Float(0.0));
    assert_eq!(b.borrow().get_length(&Value::Float(-1.0)), 0);
    assert_eq!(b.borrow().get_length(&Value::Float(0.0)), 5);
    assert_eq!(b.borrow().get_length(&Value::Float(1.0)), 10);

    // Case 3: 2.0 - 10.0
    let b = Bar::new(Value::Float(2.0), Value::Float(10.0), Value::Float(0.0));
    assert_eq!(b.borrow().get_length(&Value::Float(2.0)), 0);
    assert_eq!(b.borrow().get_length(&Value::Float(6.0)), 5);
    assert_eq!(b.borrow().get_length(&Value::Float(10.0)), 10);

    // ===
    // Int
    // ===
    // Case 1: 0 - 8
    let b = Bar::new(Value::Int(0), Value::Int(8), Value::Int(0));
    assert_eq!(b.borrow().get_length(&Value::Int(0)), 0);
    assert_eq!(b.borrow().get_length(&Value::Int(4)), 5);
    assert_eq!(b.borrow().get_length(&Value::Int(8)), 10);

    // Case 2: -4 - 4
    let b = Bar::new(Value::Int(-4), Value::Int(4), Value::Int(0));
    assert_eq!(b.borrow().get_length(&Value::Int(-4)), 0);
    assert_eq!(b.borrow().get_length(&Value::Int(0)), 5);
    assert_eq!(b.borrow().get_length(&Value::Int(4)), 10);

    // Case 3: 2 - 10
    let b = Bar::new(Value::Int(2), Value::Int(10), Value::Int(0));
    assert_eq!(b.borrow().get_length(&Value::Int(2)), 0);
    assert_eq!(b.borrow().get_length(&Value::Int(6)), 5);
    assert_eq!(b.borrow().get_length(&Value::Int(10)), 10);
}
