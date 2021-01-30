use std::cell::RefCell;
use std::hash::Hash;
use std::rc::Rc;

use super::Observer;
use super::{Value, get_int};
use super::{Printer, Widget, WidgetProperties};

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

    fn draw(&self, p: &mut dyn Printer) {
        //let value = if let Value::Int(x) = self.value { x } else { panic!() };
        let value = get_int(&self.value);
        let chars = if value > 0 { "▣" } else { "□" };
        p.set_color(self.props.colors.fg_compl_l, self.props.colors.bg_compl_l);
        p.print(self.props.pos_x, self.props.pos_y, chars);
    }
}

impl<Key: Copy + Eq + Hash> Observer for Button<Key> {
    fn update(&mut self, value: Value) {
        self.value = value;
        self.set_dirty(true);
    }
}

