use std::cell::RefCell;
use std::hash::Hash;
use std::rc::Rc;

use termion::{color, cursor};

use super::Observer;
use super::{Value, get_int, get_float};
use super::{Widget, WidgetProperties};

pub type ValueDisplayRef<Key> = Rc<RefCell<ValueDisplay<Key>>>;

/** A label displayin a numerical value. */
pub struct ValueDisplay<Key: Copy + Eq + Hash> {
    props: WidgetProperties<Key>,
    value: Value,
}

impl<Key: Copy + Eq + Hash> ValueDisplay<Key> {
    pub fn new(value: Value) -> ValueDisplayRef<Key> {
        let width = 8;
        let height = 1;
        let props = WidgetProperties::new(width, height);
        Rc::new(RefCell::new(ValueDisplay{props, value}))
    }
}

impl<Key: Copy + Eq + Hash> Widget<Key> for ValueDisplay<Key> {
    fn get_widget_properties_mut<'a>(&'a mut self) -> &'a mut WidgetProperties<Key> {
        return &mut self.props;
    }

    fn get_widget_properties<'a>(&'a self) -> &'a WidgetProperties<Key> {
        return &self.props;
    }

    fn draw(&self) {
        let v = get_int(&self.value);
        print!("{}{}{} {} ", cursor::Goto(self.props.pos_x, self.props.pos_y), color::Bg(self.props.colors.bg_dark), color::Fg(self.props.colors.fg_light), v);
    }
}

impl<Key: Copy + Eq + Hash> Observer for ValueDisplay<Key> {
    fn update(&mut self, value: Value) {
        self.value = value;
        self.set_dirty(true);
    }
}

