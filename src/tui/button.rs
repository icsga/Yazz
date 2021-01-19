use std::cell::RefCell;
use std::hash::Hash;
use std::rc::Rc;

use termion::{color, cursor};

use super::Observer;
use super::{Value, get_int};
use super::{Widget, WidgetProperties};

type ButtonRef<Key> = Rc<RefCell<Button<Key>>>;

/** A button for boolean values */
pub struct Button<Key: Copy + Eq + Hash> {
    props: WidgetProperties<Key>,
    value: Value,
}

impl<Key: Copy + Eq + Hash> Button<Key> {
    pub fn new(value: Value) -> ButtonRef<Key> {
        let width = 1;
        let height = 1;
        let props = WidgetProperties::new(width, height);
        Rc::new(RefCell::new(Button{props, value}))
    }
}

impl<Key: Copy + Eq + Hash> Widget<Key> for Button<Key> {
    fn get_widget_properties_mut(&mut self) -> &mut WidgetProperties<Key> {
        &mut self.props
    }

    fn get_widget_properties(&self) -> &WidgetProperties<Key> {
        &self.props
    }

    fn draw(&self) {
        //let value = if let Value::Int(x) = self.value { x } else { panic!() };
        let value = get_int(&self.value);
        let chars = if value > 0 { "▣" } else { "□" };
        print!("{}{}{}{}", cursor::Goto(self.props.pos_x, self.props.pos_y), color::Bg(self.props.colors.bg_light2), color::Fg(self.props.colors.fg_dark2), chars);
    }
}

impl<Key: Copy + Eq + Hash> Observer for Button<Key> {
    fn update(&mut self, value: Value) {
        self.value = value;
        self.set_dirty(true);
    }
}

