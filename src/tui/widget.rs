use std::cell::RefCell;
use std::hash::Hash;
use std::rc::Rc;

//use super::MouseMessage;
use super::ColorScheme;
use super::{Index, Printer};

pub type WidgetRef<T> = Rc<RefCell<dyn Widget<T>>>;

#[derive(Debug)]
pub struct WidgetProperties<Key: Copy + Eq + Hash> {
    key: Option<Key>,
    pub pos_x: Index,
    pub pos_y: Index,
    pub width: Index,
    pub height: Index,
    pub dirty: bool,
    pub colors: Rc<ColorScheme>,
}

impl<Key: Copy + Eq + Hash> WidgetProperties<Key> {
    pub fn new(width: Index, height: Index) -> WidgetProperties<Key> {
        let pos_x: Index = 0;
        let pos_y: Index = 0;
        let dirty = false;
        let colors = Rc::new(ColorScheme::new());
        WidgetProperties{key: None, pos_x, pos_y, width, height, dirty, colors}
    }

    pub fn set_position(&mut self, x: Index, y: Index) -> bool {
         // TODO: Check that new position is valid
        self.pos_x = x;
        self.pos_y = y;
        true
    }

    pub fn set_width(&mut self, width: Index) -> bool {
         // TODO: Check that width is valid
        self.width = width;
        true
    }

    pub fn set_height(&mut self, height: Index) -> bool {
         // TODO: Check that height is valid
        self.height = height;
        true
    }

    pub fn set_dirty(&mut self, is_dirty: bool) {
        self.dirty = is_dirty;
    }

    pub fn set_color_scheme(&mut self, colors: Rc<ColorScheme>) {
        self.colors = colors;
    }

    pub fn set_key(&mut self, key: Key) {
        self.key = Some(key);
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn get_position(&self) -> (Index, Index) {
        (self.pos_x, self.pos_y)
    }

    pub fn get_width(&self) -> Index {
        self.width
    }

    pub fn get_height(&self) -> Index {
        self.height
    }

    pub fn get_size(&self) -> (Index, Index) {
        (self.get_width(), self.get_height())
    }

    pub fn is_inside(&self, x: Index, y: Index) -> bool {
        x >= self.pos_x &&
        x <= self.pos_x + self.width &&
        y >= self.pos_y &&
        y <= self.pos_y + self.height
    }

    /*
    pub fn get_mouse_offset(&self, msg: &MouseMessage) -> (Index, Index) {
    }
    */
}

pub trait Widget<Key: Copy + Eq + Hash> {

    // -------------------------------------------
    // Must be implemented by the deriving Widgets

    fn get_widget_properties_mut(&mut self) -> &mut WidgetProperties<Key>;
    fn get_widget_properties(&self) -> &WidgetProperties<Key>;
    fn draw(&self, printer: &mut dyn Printer);

    // -------------------------------------------

    // -------------------------------------------
    // Default implementations forward to WidgetProperties

    fn set_position(&mut self, x: Index, y: Index) -> bool {
        return self.get_widget_properties_mut().set_position(x, y);
    }

    fn set_width(&mut self, width: Index) -> bool {
        return self.get_widget_properties_mut().set_width(width);
    }

    fn set_height(&mut self, height: Index) -> bool {
        return self.get_widget_properties_mut().set_height(height);
    }

    fn set_dirty(&mut self, is_dirty: bool) {
        self.get_widget_properties_mut().set_dirty(is_dirty);
    }

    fn set_color_scheme(&mut self, colors: Rc<ColorScheme>) {
        self.get_widget_properties_mut().set_color_scheme(colors);
    }

    fn set_key(&mut self, key: Key) {
        self.get_widget_properties_mut().set_key(key);
    }

    fn is_dirty(&self) -> bool {
        return self.get_widget_properties().is_dirty();
    }

    fn is_inside(&self, x: Index, y: Index) -> bool {
        return self.get_widget_properties().is_inside(x, y);
    }

    fn get_position(&self) -> (Index, Index) { // x, y
        return self.get_widget_properties().get_position();
    }

    fn get_key(&self) -> Option<Key> {
        self.get_widget_properties().key
    }

    fn get_at_pos(&self, x: Index, y: Index) -> Option<Key> {
        if self.is_inside(x, y) {
            self.get_key()
        } else {
            None
        }
    }

    fn get_width(&self) -> Index {
        return self.get_widget_properties().get_width();
    }

    fn get_height(&self) -> Index {
        return self.get_widget_properties().get_height();
    }

    fn get_size(&self) -> (Index, Index) { // width, height
        return self.get_widget_properties().get_size();
    }

    // -------------------------------------------
}
