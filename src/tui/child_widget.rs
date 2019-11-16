use std::cell::RefCell;
use std::rc::Rc;

use super::Index;
use super::Scheme;
use super::{Widget, WidgetProperties};

pub struct ChildWidget {
    pos_x: Index, // Relative position inside a container
    pos_y: Index, // Relative position inside a container
    child: Rc<RefCell<dyn Widget>>,
}

impl ChildWidget {
    pub fn new<W: Widget + 'static>(child: Rc<RefCell<W>>, pos_x: Index, pos_y: Index) -> ChildWidget {
        ChildWidget{pos_x, pos_y, child}
    }

    /** Update the relative position of the ChildWidget in the Container.
     *
     * TODO: Check the position is valid wrt. child size, position
     */
    pub fn move_child(&mut self, pos_x: Index, pos_y: Index) -> bool {
        self.pos_x = pos_x;
        self.pos_y = pos_y;
        true
    }
}

impl Widget for ChildWidget {
    fn get_widget_properties<'a>(&'a mut self) -> &'a mut WidgetProperties {
        return self.child.borrow_mut().get_widget_properties();
    }

    /** Calculate and update the position of the child.
     *
     * We expect the Container to pass it's own position.
     */
    fn set_position(&mut self, x: Index, y: Index) -> bool {
        let child_x = x + self.pos_x;
        let child_y = y + self.pos_y;
        self.child.borrow_mut().set_position(child_x, child_y)
    }

    fn set_height(&mut self, height: Index) -> bool {
        self.child.borrow_mut().set_height(height)
    }

    fn set_width(&mut self, width: Index) -> bool {
        self.child.borrow_mut().set_width(width)
    }

    fn set_dirty(&mut self, is_dirty: bool) {
        self.child.borrow_mut().set_dirty(is_dirty);
    }

    fn set_color_scheme(&mut self, colors: Rc<Scheme>) {
        self.child.borrow_mut().set_color_scheme(colors);
    }

    fn is_dirty(&self) -> bool {
        self.child.borrow().is_dirty()
    }

    /** Get absolute position of child. */
    fn get_position(&self) -> (Index, Index) {
        self.child.borrow().get_position()
    }

    fn get_size(&self) -> (Index, Index) {
        self.child.borrow().get_size()
    }

    fn draw(&self) {
        self.child.borrow().draw();
    }
}
