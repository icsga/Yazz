use std::cell::RefCell;
use std::rc::Rc;

use super::ChildWidget;
use super::Index;
use super::Scheme;
use super::{Widget, WidgetProperties};

pub type ContainerRef = Rc<RefCell<Container>>;

pub struct Container {
    props: WidgetProperties,
    children: Vec<ChildWidget>,
}

impl Container {
    pub fn new(width: Index, height: Index) -> Container {
        let props = WidgetProperties::new(width, height);
        let children = vec!{};
        Container{props, children}
    }

    pub fn add_child<C: Widget + 'static>(&mut self, child: Rc<RefCell<C>>, pos_x: Index, pos_y: Index) {
        let mut cw = ChildWidget::new(child, pos_x, pos_y); // Sets relative position in the child widget
        cw.set_position(self.props.pos_x, self.props.pos_y); // Update child with current absolute position
        self.children.push(cw);
    }
}

impl Widget for Container {
    // TODO: Implement dynamic resizing of children

    fn get_widget_properties<'a>(&'a mut self) -> &'a mut WidgetProperties {
        return &mut self.props;
    }

    /** Set the container's and all its children's position.
     *
     * TODO: Check that new position is valid
     */
    fn set_position(&mut self, x: Index, y: Index) -> bool {
        self.props.set_position(x, y);
        for child in self.children.iter_mut() {
            child.set_position(x, y);
        }
        true
    }

    fn set_color_scheme(&mut self, colors: Rc<Scheme>) {
        for c in self.children.iter_mut() {
            c.set_color_scheme(colors.clone());
        }
    }

    fn is_dirty(&self) -> bool {
        for c in self.children.iter() {
            if c.is_dirty() {
                return true;
            }
        }
        false
    }

    fn draw(&self) {
        for child in self.children.iter() {
            child.draw();
        }
    }
}
