use std::cell::RefCell;
use std::hash::Hash;
use std::rc::Rc;

use termion::{color, cursor};

use super::Index;
use super::Scheme;
use super::{Widget, WidgetProperties, WidgetRef};

pub type ContainerRef<Key> = Rc<RefCell<Container<Key>>>;

pub struct Container<Key: Copy + Eq + Hash> {
    props: WidgetProperties<Key>,
    draw_border: bool,
    children: Vec<WidgetRef<Key>>,
}

impl<Key: Copy + Eq + Hash> Container<Key> {
    pub fn new() -> Container<Key> {
        let props = WidgetProperties::new(0, 0);
        let draw_border = false;
        let children = vec!{};
        Container{props, draw_border, children}
    }

    pub fn add_child<C: Widget<Key> + 'static>(&mut self, child: Rc<RefCell<C>>, pos_x: Index, pos_y: Index) {
        // Check if we need to leave space for drawing the border
        let x_offset = if self.draw_border { 1 } else { 0 };
        let y_offset = if self.draw_border { 1 } else { 0 };
        // Update child with current absolute position
        child.borrow_mut().set_position(self.props.pos_x + pos_x + x_offset,
                                        self.props.pos_y + pos_y + y_offset);
        let (child_width, child_height) = child.borrow().get_size();
        let x_size = pos_x + child_width + x_offset * 2;
        //let y_size = pos_y + child_height + y_offset * 2;
        let y_size = pos_y + child_height + y_offset;
        let (width, height) = self.props.get_size();
        if x_size > width {
            self.props.set_width(x_size);
        }
        if y_size > height {
            self.props.set_height(y_size);
        }
        self.children.push(child);
    }

    pub fn enable_border(&mut self, enable: bool) {
        self.draw_border = enable;
    }

    fn draw_border(&self) {
        print!("{}{}{}┌", cursor::Goto(self.props.pos_x, self.props.pos_y), color::Bg(self.props.colors.bg_light), color::Fg(self.props.colors.fg_dark));
        let x_start = self.props.pos_x;
        let x_end = x_start + self.props.width;
        let y_start = self.props.pos_y;
        let y_end = y_start + self.props.height;
        for x in  (x_start + 1)..(x_end) {
            print!("─");
        }
        print!("┐");
        for y in (y_start + 1)..(y_end) {
            print!("{}│{}│", cursor::Goto(x_start, y), cursor::Goto(x_end, y));
        }
        print!("{}└", cursor::Goto(x_start, y_end));
        for x in  (x_start + 1)..(x_end) {
            print!("─");
        }
        print!("┘");
    }

    /** Get widget at given position. */
    pub fn get_at_pos(&self, x: Index, y: Index) -> Option<Key> {
        if self.is_inside(x, y) {
            for c in self.children.iter() {
                let result = c.borrow().get_at_pos(x, y);
                if let Some(key) = result { return result; };
            }
        }
        return None;
    }
}

impl<Key: Copy + Eq + Hash> Widget<Key> for Container<Key> {
    // TODO: Implement dynamic resizing of children

    fn get_widget_properties_mut<'a>(&'a mut self) -> &'a mut WidgetProperties<Key> {
        return &mut self.props;
    }

    fn get_widget_properties<'a>(&'a self) -> &'a WidgetProperties<Key> {
        return &self.props;
    }

    /** Set the container's and all its children's position.
     *
     * TODO: Check that new position is valid
     */
    fn set_position(&mut self, x: Index, y: Index) -> bool {
        let (x_old, y_old) = self.props.get_position();
        let x_diff = (x as i32) - (x_old as i32);
        let y_diff = (y as i32) - (y_old as i32);
        self.props.set_position(x, y);
        for child in self.children.iter_mut() {
            let (x_child, y_child) = child.borrow().get_position();
            let x_new = (x_child as i32) + x_diff;
            let y_new = (y_child as i32) + y_diff;
            child.borrow_mut().set_position(x_new as Index, y_new as Index);
        }
        true
    }

    fn set_color_scheme(&mut self, colors: Rc<Scheme>) {
        for c in self.children.iter_mut() {
            c.borrow_mut().set_color_scheme(colors.clone());
        }
    }

    fn set_dirty(&mut self, is_dirty: bool) {
        for child in self.children.iter() {
            child.borrow_mut().set_dirty(is_dirty);
        }
    }

    fn is_dirty(&self) -> bool {
        for child in self.children.iter() {
            if child.borrow().is_dirty() {
                return true;
            }
        }
        false
    }

    fn draw(&self) {
        if self.draw_border {
            self.draw_border();
        }
        for child in self.children.iter() {
            if child.borrow().is_dirty() {
                child.borrow_mut().draw();
            }
        }
    }
}
