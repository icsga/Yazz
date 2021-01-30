use super::Index;
use super::Printer;
use super::ColorScheme;
use super::{Widget, WidgetProperties, WidgetRef};

use std::cell::RefCell;
use std::hash::Hash;
use std::rc::Rc;
use std::fmt;

pub type ContainerRef<Key> = Rc<RefCell<Container<Key>>>;

pub struct Container<Key: Copy + Eq + Hash> {
    title: String,
    props: WidgetProperties<Key>,
    draw_border: bool,
    children: Vec<WidgetRef<Key>>,
}

impl<T: Copy + Eq + Hash> fmt::Debug for Container<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Container")
         .field("title", &self.title)
         .field("props.x", &self.props.pos_x)
         .field("props.y", &self.props.pos_y)
         .field("props.width", &self.props.width)
         .field("props.height", &self.props.height)
         .field("draw_border", &self.draw_border)
         .finish()
    }
}

impl<Key: Copy + Eq + Hash> Container<Key> {
    pub fn new() -> Container<Key> {
        let title = "".to_string();
        let props = WidgetProperties::new(0, 0);
        let draw_border = false;
        let children = vec!{};
        Container{title, props, draw_border, children}
    }

    pub fn add_child<C: Widget<Key> + 'static>(&mut self, child: Rc<RefCell<C>>, pos_x: Index, pos_y: Index) {
        // Check if we need to leave space for drawing the border
        let (x_offset, y_offset) = if self.draw_border { (1, 1) } else { (0, 0) };
        // Update child with current absolute position
        child.borrow_mut().set_position(self.props.pos_x + pos_x + x_offset,
                                        self.props.pos_y + pos_y + y_offset);
        let (child_width, child_height) = child.borrow().get_size();
        let x_size = pos_x + child_width + x_offset * 2;
        let y_size = pos_y + child_height + y_offset * 2;
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
        if enable && !self.draw_border {
            self.props.set_width(self.props.get_width() + 2);
            self.props.set_height(self.props.get_height() + 2);
        } else if !enable && self.draw_border {
            self.props.set_width(self.props.get_width() - 2);
            self.props.set_height(self.props.get_height() - 2);
        }
        self.draw_border = enable;
    }

    pub fn set_title(&mut self, title: String) {
        self.title = format!("┤ {} ├", title);
    }

    fn draw_border(&self, p: &mut dyn Printer) {
        p.set_color(self.props.colors.fg_base, self.props.colors.bg_base); 
        p.print(self.props.pos_x, self.props.pos_y, "┌");

        // Overall position of frame
        let x_start = self.props.pos_x;
        let x_end = x_start + self.props.width;
        let y_start = self.props.pos_y;
        let y_end = y_start + self.props.height - 1;

        // Calculate position and width of container title, if any
        let title_len = if self.title.len() > 4 { self.title.len() - 4 } else { 0 }; // Unicode chars have wrong len. TODO: Handle case where title is longer than width
        let x_middle_left = x_start + (self.props.width / 2) - (title_len / 2) as Index;
        let mut x_middle_right = x_start + (self.props.width / 2) + (title_len / 2) as Index;
        if (x_middle_left - x_start) + title_len as Index + (x_end - x_middle_right) > self.props.width {
            x_middle_right += 1;
        }

        // Draw upper line and title
        for x in  (x_start + 1)..(x_middle_left) {
            p.print(x, self.props.pos_y, "─");
        }
        p.print(x_middle_left, self.props.pos_y, &self.title);
        for x in  (x_middle_right)..(x_end) {
            p.print(x, self.props.pos_y, "─");
        }

        p.print(x_end, self.props.pos_y, "┐");
        for y in (y_start + 1)..(y_end) {
            p.print(x_start, y, "│");
            p.print(x_end, y, "│");
        }
        p.print(x_start, y_end, "└");
        for x in  (x_start + 1)..(x_end) {
            p.print(x, y_end, "─");
        }
        p.print(x_end, y_end, "┘");
    }

    /** Get widget at given position. */
    pub fn get_at_pos(&self, x: Index, y: Index) -> Option<Key> {
        if self.is_inside(x, y) {
            for c in self.children.iter() {
                let result = c.borrow().get_at_pos(x, y);
                if result.is_some() { return result; };
            }
        }
        None
    }
}

impl<Key: Copy + Eq + Hash> Widget<Key> for Container<Key> {
    // TODO: Implement dynamic resizing of children

    fn get_widget_properties_mut(&mut self) -> &mut WidgetProperties<Key> {
        &mut self.props
    }

    fn get_widget_properties(&self) -> &WidgetProperties<Key> {
        &self.props
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

    fn set_color_scheme(&mut self, colors: Rc<ColorScheme>) {
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

    fn draw(&self, printer: &mut dyn Printer) {
        if self.draw_border {
            self.draw_border(printer);
        }
        for child in self.children.iter() {
            if child.borrow().is_dirty() {
                child.borrow_mut().draw(printer);
            }
        }
    }
}

// ----------
// Unit tests
// ----------

#[cfg(test)]

use super::{Label}; // Needed for tests. TODO: There's got to be a better way

fn validate_properties<T: Copy + Eq + Hash>(c: &Container<T>, pos_x: Index, pos_y: Index, width: Index, height: Index) -> bool {
    if c.props.pos_x == pos_x
    && c.props.pos_y == pos_y
    && c.props.width == width
    && c.props.height == height {
        true
    } else {
        println!("\n{:?}", c);
        false
    }
}

#[test]
fn size_changes_when_child_added_no_border() {
    let mut c: Container<i32> = Container::new();
    assert!(validate_properties(&c, 0, 0, 0, 0));
    c.add_child(Label::new("12345".to_string(), 5), 0, 0);
    assert!(validate_properties(&c, 0, 0, 5, 1));
}

#[test]
fn size_changes_when_child_added_border() {
    let mut c: Container<i32> = Container::new();
    c.enable_border(true);
    assert!(validate_properties(&c, 0, 0, 2, 2));
    c.add_child(Label::new("12345".to_string(), 5), 0, 0);
    assert!(validate_properties(&c, 0, 0, 7, 3));
    c.add_child(Label::new("12345".to_string(), 5), 0, 1);
    assert!(validate_properties(&c, 0, 0, 7, 4));
}
