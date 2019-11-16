use std::cell::RefCell;
use std::rc::Rc;

use termion::{color, cursor};

use super::Index;
use super::Observer;
use super::{Value, get_str};
use super::{Widget, WidgetProperties};

type LabelRef = Rc<RefCell<Label>>;

pub struct Label {
    props: WidgetProperties,
    value: Value,
}

impl Label {
    pub fn new(value: String, size: Index) -> LabelRef {
        let width = size;
        let height = 1;
        let props = WidgetProperties::new(width, height);
        let value = Value::Str(value);
        Rc::new(RefCell::new(Label{props, value}))
    }
}

impl Widget for Label {
    fn get_widget_properties<'a>(&'a mut self) -> &'a mut WidgetProperties {
        return &mut self.props;
    }

    fn draw(&self) {
        print!("{}{}{}{}",
            cursor::Goto(self.props.pos_x, self.props.pos_y),
            color::Bg(self.props.colors.bg_light),
            color::Fg(self.props.colors.fg_dark),
            get_str(&self.value));
    }
}

impl Observer for Label {
    fn update(&mut self, value: Value) {
        self.value = value;
        self.set_dirty(true);
    }
}
