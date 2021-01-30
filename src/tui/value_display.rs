use std::cell::RefCell;
use std::hash::Hash;
use std::rc::Rc;

use termion::cursor;

use super::Observer;
use super::{Value, get_int};
use super::{Printer, Widget, WidgetProperties};

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
    fn get_widget_properties_mut(&mut self) -> &mut WidgetProperties<Key> {
        &mut self.props
    }

    fn get_widget_properties(&self) -> &WidgetProperties<Key> {
        &self.props
    }

    fn draw(&self, p: &mut dyn Printer) {
        let v = get_int(&self.value);
        p.set_color(self.props.colors.fg_compl, self.props.colors.bg_compl);
        print!("{} {} ", cursor::Goto(self.props.pos_x as u16, self.props.pos_y as u16), v);
    }
}

impl<Key: Copy + Eq + Hash> Observer for ValueDisplay<Key> {
    fn update(&mut self, value: Value) {
        self.value = value;
        self.set_dirty(true);
    }
}

