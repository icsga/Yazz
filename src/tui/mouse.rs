use super::{Container, Controller, Index};

use log::info;

use std::cmp::Eq;
use std::hash::Hash;

#[derive(Debug)]
enum MhState {
    Idle,
    Clicked
}

#[derive(Debug)]
pub enum MouseMessage {
    Press{x: Index, y: Index},
    Hold{x: Index, y: Index},
    Release{x: Index, y: Index},
}

pub struct MouseHandler<Key: Copy + Eq + Hash> {
    state: MhState,
    current_key: Option<Key>,
}

impl<Key: Copy + Eq + Hash> MouseHandler<Key> {
    pub fn new() -> MouseHandler<Key> {
        let current_key = None;
        MouseHandler{state: MhState::Idle, current_key}
    }

    pub fn handle_event(&mut self, msg: &MouseMessage, window: &Container<Key>, controller: &Controller<Key>) {
        match self.state {
            MhState::Idle => self.idle_state(msg, window, controller),
            MhState::Clicked => self.clicked_state(msg, window, controller),
        }
    }

    fn idle_state(&mut self, msg: &MouseMessage, window: &Container<Key>, _controller: &Controller<Key>) {
        match msg {
            MouseMessage::Press{x, y} => {
                // Select widget to be updated
                self.change_state(MhState::Clicked);
                self.current_key = window.get_at_pos(*x, *y);
                //controller.handle_mouse_event(&self.current_key.unwrap(), msg);
            },
            MouseMessage::Hold{x: _, y: _} | MouseMessage::Release{x: _, y: _} => {},
        }
    }

    fn clicked_state(&mut self, msg: &MouseMessage, _window: &Container<Key>, _controller: &Controller<Key>) {
        match msg {
            MouseMessage::Press{x: _, y: _} => {},
            MouseMessage::Hold{x: _, y: _} => {
                // Update selected widget
            },
            MouseMessage::Release{x: _, y: _} => {
                // Finished with widget value change
                self.change_state(MhState::Idle);
            },
        }
    }

    fn change_state(&mut self, new_state: MhState) {
        info!("Mouse: Change state {:?} -> {:?}", self.state, new_state);
        self.state = new_state;
    }
}
