use std::cell::RefCell;
use std::rc::Rc;

use super::ChildWidget;
use super::Index;
use super::Widget;

pub type ContainerRef = Rc<RefCell<Container>>;

pub struct Container {
    pos_x: Index,
    pos_y: Index,
    width: Index,
    height: Index,
    dirty: bool,
    children: Vec<ChildWidget>,
}

impl Container {
    pub fn new(width: Index, height: Index) -> Container {
        let pos_x: Index = 0;
        let pos_y: Index = 0;
        let dirty = true;
        let children = vec!{};
        Container{pos_x, pos_y, width, height, dirty, children}
    }

    pub fn add_child<C: Widget + 'static>(&mut self, child: Rc<RefCell<C>>, pos_x: Index, pos_y: Index) {
        let mut cw = ChildWidget::new(child, pos_x, pos_y); // Sets relative position in the child widget
        cw.set_position(self.pos_x, self.pos_y); // Update child with current absolute position
        self.children.push(cw);
    }
}

impl Widget for Container {

    /** Set the container's and all its children's position.
     *
     * TODO: Check that new position is valid
     */
    fn set_position(&mut self, x: Index, y: Index) -> bool {
        self.pos_x = x;
        self.pos_y = y;
        for child in self.children.iter_mut() {
            child.set_position(x, y);
        }
        true
    }

    /** Set container's width.
     *
     * TODO: - Update children's width if dynamic
     *       - Check that children can still be displayed
     */
    fn set_width(&mut self, width: Index) -> bool {
        self.width = width;
        true
    }

    /** Set container's height.
     *
     * TODO: - Update children's height if dynamic
     *       - Check that children can still be displayed
     */
    fn set_height(&mut self, height: Index) -> bool {
        self.height = height;
        true
    }

    fn set_dirty(&mut self, is_dirty: bool) {
        self.dirty = is_dirty;
    }

    fn is_dirty(&self) -> bool {
        for c in self.children.iter() {
            if c.is_dirty() {
                return true;
            }
        }
        false
    }

    fn get_position(&self) -> (Index, Index) {
        (self.pos_x, self.pos_y)
    }

    fn get_size(&self) -> (Index, Index) {
        (self.width, self.height)
    }

    fn draw(&self) {
        for child in self.children.iter() {
            child.draw();
        }
    }
}
