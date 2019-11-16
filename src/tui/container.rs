use std::cell::RefCell;
use std::rc::Rc;

use super::Index;
use super::Scheme;
use super::{Widget, WidgetProperties, WidgetRef};

pub type ContainerRef = Rc<RefCell<Container>>;

pub struct Container {
    props: WidgetProperties,
    children: Vec<WidgetRef>,
}

impl Container {
    pub fn new(width: Index, height: Index) -> Container {
        let props = WidgetProperties::new(width, height);
        let children = vec!{};
        Container{props, children}
    }

    pub fn add_child<C: Widget + 'static>(&mut self, child: Rc<RefCell<C>>, pos_x: Index, pos_y: Index) {
        child.borrow_mut().set_position(self.props.pos_x + pos_x, self.props.pos_y + pos_y); // Update child with current absolute position
        self.children.push(child);
    }
}

impl Widget for Container {
    // TODO: Implement dynamic resizing of children

    fn get_widget_properties_mut<'a>(&'a mut self) -> &'a mut WidgetProperties {
        return &mut self.props;
    }

    fn get_widget_properties<'a>(&'a self) -> &'a WidgetProperties {
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

    fn is_dirty(&self) -> bool {
        for c in self.children.iter() {
            if c.borrow().is_dirty() {
                return true;
            }
        }
        false
    }

    fn draw(&self) {
        for child in self.children.iter() {
            child.borrow_mut().draw();
        }
    }
}
