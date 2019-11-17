use std::cell::RefCell;
use std::rc::Rc;

use termion::{color, cursor};

use super::Observer;
use super::{Value, get_int, get_float};
use super::{Widget, WidgetProperties};

pub type ValueDisplayRef = Rc<RefCell<ValueDisplay>>;

/** A label displayin a numerical value. */
pub struct ValueDisplay {
    props: WidgetProperties,
    value: Value,
}

impl ValueDisplay {
    pub fn new(value: Value) -> ValueDisplayRef {
        let width = 8;
        let height = 1;
        let props = WidgetProperties::new(width, height);
        Rc::new(RefCell::new(ValueDisplay{props, value}))
    }
}

impl Widget for ValueDisplay {
    fn get_widget_properties_mut<'a>(&'a mut self) -> &'a mut WidgetProperties {
        return &mut self.props;
    }

    fn get_widget_properties<'a>(&'a self) -> &'a WidgetProperties {
        return &self.props;
    }

    fn draw(&self) {
        let v = get_int(&self.value);
        print!("{}{}{} {} ", cursor::Goto(self.props.pos_x, self.props.pos_y), color::Bg(self.props.colors.bg_dark), color::Fg(self.props.colors.fg_light), v);
    }
}

impl Observer for ValueDisplay {
    fn update(&mut self, value: Value) {
        self.value = value;
        self.set_dirty(true);
    }
}

