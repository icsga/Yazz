use std::cell::RefCell;
use std::rc::Rc;
use std::cmp::Eq;
use std::hash::Hash;

use super::{Value, Widget};

pub type ObserverRef = Rc<RefCell<dyn Observer>>;

pub trait Observer {
    fn update(&mut self, value: Value);

    //fn handle_mouse_event(&mut self, msg: &MouseMessage);
}
