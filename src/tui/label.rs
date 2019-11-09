use std::cell::RefCell;
use std::rc::Rc;

use termion::{color, cursor};

use super::Index;
use super::Observer;
use super::Scheme;
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
    colors: Rc<Scheme>,
}

impl Label {
    pub fn new(value: &'static str, size: Index) -> LabelRef {
        let pos_x: Index = 0;
        let pos_y: Index = 0;
        let width = size;
        let height = 1;
        let value = Value::Str(value);
        let dirty = false;
        let colors = Rc::new(Scheme::new());
        Rc::new(RefCell::new(Label{pos_x, pos_y, width, height, value, dirty, colors}))
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
        print!("{}{}{}{}", cursor::Goto(self.pos_x, self.pos_y), color::Bg(self.colors.bg_light), color::Fg(self.colors.fg_dark), get_str(&self.value));
    }
}

impl Observer for Label {
    fn update(&mut self, value: Value) {
        self.value = value;
        self.set_dirty(true);
    }
}
