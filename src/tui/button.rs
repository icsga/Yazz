use std::cell::RefCell;
use std::rc::Rc;

use termion::{color, cursor};

use super::Observer;
use super::{Value, get_int};
use super::{Widget, WidgetProperties};

type ButtonRef = Rc<RefCell<Button>>;

/** A button for boolean values */
pub struct Button {
    props: WidgetProperties,
    value: Value,
}

impl Button {
    pub fn new(value: Value) -> ButtonRef {
        let width = 1;
        let height = 1;
        let props = WidgetProperties::new(width, height);
        Rc::new(RefCell::new(Button{props, value}))
    }
}

impl Widget for Button {
    fn get_widget_properties_mut<'a>(&'a mut self) -> &'a mut WidgetProperties {
        return &mut self.props;
    }

    fn get_widget_properties<'a>(&'a self) -> &'a WidgetProperties {
        return &self.props;
    }

    fn draw(&self) {
        //let value = if let Value::Int(x) = self.value { x } else { panic!() };
        let value = get_int(&self.value);
        let chars = if value > 0 { "▣" } else { "□" };
        print!("{}{}{}{}", cursor::Goto(self.props.pos_x, self.props.pos_y), color::Bg(self.props.colors.bg_light2), color::Fg(self.props.colors.fg_dark2), chars);
    }
}

impl Observer for Button {
    fn update(&mut self, value: Value) {
        self.value = value;
        self.set_dirty(true);
    }
}

