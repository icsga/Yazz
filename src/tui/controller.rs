use std::collections::HashMap;
use std::rc::Rc;
use std::cell::RefCell;
use std::cmp::Eq;
use std::hash::Hash;

//use log::{info, trace, warn};

use super::Observer;
use super::Value;

pub struct Controller<Key: Copy + Eq + Hash> {
    pub observers: HashMap<Key, Rc<RefCell<dyn Observer>>>,
}

impl<Key: Copy + Eq + Hash> Controller<Key> {
    pub fn new() -> Controller<Key> {
        let observers = HashMap::new();
        Controller{observers}
    }

    pub fn add_observer(&mut self, key: &Key, observer: Rc<RefCell<dyn Observer>>) {
        self.observers.insert(*key, observer);
    }

    pub fn update(&mut self, key: &Key, value: Value) {
        self.observers.entry(*key).and_modify(|e| e.borrow_mut().update(value));
    }
}

