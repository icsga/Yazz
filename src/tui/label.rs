use std::cell::RefCell;
use std::hash::Hash;
use std::rc::Rc;

use super::Index;
use super::Observer;
use super::{Value, get_str};
use super::{Printer, Widget, WidgetProperties};

type LabelRef<Key> = Rc<RefCell<Label<Key>>>;

pub struct Label<Key: Copy + Eq + Hash> {
    props: WidgetProperties<Key>,
    value: Value,
    light: bool,
}

impl<Key: Copy + Eq + Hash> Label<Key> {
    pub fn new(value: String, size: Index) -> LabelRef<Key> {
        let width = size;
        let height = 1;
        let props = WidgetProperties::new(width, height);
        let value = Value::Str(value);
        let light = false;
        Rc::new(RefCell::new(Label{props, value, light}))
    }

    pub fn select_light(&mut self) {
        self.light = true;
    }
}

impl<Key: Copy + Eq + Hash> Widget<Key> for Label<Key> {
    fn get_widget_properties_mut(&mut self) -> &mut WidgetProperties<Key> {
        &mut self.props
    }

    fn get_widget_properties(&self) -> &WidgetProperties<Key> {
        &self.props
    }

    fn draw(&self, p: &mut dyn Printer) {
        if self.light {
            p.set_color(self.props.colors.fg_base_l, self.props.colors.bg_base);
        } else {
            p.set_color(self.props.colors.fg_base, self.props.colors.bg_base);
        }
        p.print(self.props.pos_x, self.props.pos_y, get_str(&self.value));
    }
}

impl<Key: Copy + Eq + Hash> Observer for Label<Key> {
    fn update(&mut self, value: Value) {
        self.value = value;
        self.set_dirty(true);
    }
}
