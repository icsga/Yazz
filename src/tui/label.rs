use std::cell::RefCell;
use std::rc::Rc;

use termion::{color, cursor};
use termion::color::{Black, White};

use super::Index;
use super::Observer;
use super::{Value, get_str};
use super::Widget;

type LabelRef = Rc<RefCell<Label>>;

pub struct Label {
    pos_x: Index,
    pos_y: Index,
    width: Index,
    height: Index,
    value: Value,
    dirty: bool,
}

impl Label {
    pub fn new(value: &'static str) -> LabelRef {
        let pos_x: Index = 0;
        let pos_y: Index = 0;
        let width = value.len() as Index;
        let height = 1;
        let value = Value::Str(value);
        let dirty = false;
        Rc::new(RefCell::new(Label{pos_x, pos_y, width, height, value, dirty}))
    }
}

impl Widget for Label {
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
        print!("{}{}{}{}", cursor::Goto(self.pos_x, self.pos_y), color::Bg(White), color::Fg(Black), get_str(&self.value));
    }
}

impl Observer for Label {
    fn update(&mut self, value: Value) {
        self.value = value;
        self.set_dirty(true);
    }
}
