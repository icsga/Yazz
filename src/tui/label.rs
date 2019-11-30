use std::cell::RefCell;
use std::hash::Hash;
use std::rc::Rc;

use termion::{color, cursor};

use super::Index;
use super::Observer;
use super::{Value, get_str};
use super::{Widget, WidgetProperties};

type LabelRef<Key> = Rc<RefCell<Label<Key>>>;

pub struct Label<Key: Copy + Eq + Hash> {
    props: WidgetProperties<Key>,
    value: Value,
}

impl<Key: Copy + Eq + Hash> Label<Key> {
    pub fn new(value: String, size: Index) -> LabelRef<Key> {
        let width = size;
        let height = 1;
        let props = WidgetProperties::new(width, height);
        let value = Value::Str(value);
        Rc::new(RefCell::new(Label{props, value}))
    }
}

impl<Key: Copy + Eq + Hash> Widget<Key> for Label<Key> {
    fn get_widget_properties_mut<'a>(&'a mut self) -> &'a mut WidgetProperties<Key> {
        return &mut self.props;
    }

    fn get_widget_properties<'a>(&'a self) -> &'a WidgetProperties<Key> {
        return &self.props;
    }

    fn draw(&self) {
        print!("{}{}{}{}",
            cursor::Goto(self.props.pos_x, self.props.pos_y),
            color::Bg(self.props.colors.bg_light),
            color::Fg(self.props.colors.fg_dark),
            get_str(&self.value));
    }
}

impl<Key: Copy + Eq + Hash> Observer for Label<Key> {
    fn update(&mut self, value: Value) {
        self.value = value;
        self.set_dirty(true);
    }
}
