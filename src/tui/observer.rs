use std::cell::RefCell;
use std::rc::Rc;

use super::Value;

pub type ObserverRef = Rc<RefCell<dyn Observer>>;

pub trait Observer {
    fn update(&mut self, value: Value);

    //fn handle_mouse_event(&mut self, msg: &MouseMessage);
}
