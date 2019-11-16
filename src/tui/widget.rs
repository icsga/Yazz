use std::rc::Rc;

use super::Scheme;

pub type Index = u16;

pub struct WidgetProperties {
    pub pos_x: Index,
    pub pos_y: Index,
    pub width: Index,
    pub height: Index,
    pub dirty: bool,
    pub colors: Rc<Scheme>,
}

impl WidgetProperties {
    pub fn new(width: Index, height: Index) -> WidgetProperties {
        let pos_x: Index = 0;
        let pos_y: Index = 0;
        let dirty = false;
        let colors = Rc::new(Scheme::new());
        WidgetProperties{pos_x, pos_y, width, height, dirty, colors}
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

    pub fn set_color_scheme(&mut self, colors: Rc<Scheme>) {
        self.colors = colors;
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn get_position(&self) -> (Index, Index) {
        (self.pos_x, self.pos_y)
    }

    pub fn get_size(&self) -> (Index, Index) {
        (self.width, self.height)
    }
}

pub trait Widget {

    // -------------------------------------------
    // Must be implemented by the deriving Widgets

    fn get_widget_properties<'a>(&'a mut self) -> &'a mut WidgetProperties;

    fn draw(&self);

    // -------------------------------------------

    // -------------------------------------------
    // Default implementations forward to WidgetProperties

    fn set_position(&mut self, x: Index, y: Index) -> bool {
        return self.get_widget_properties().set_position(x, y);
    }

    fn set_width(&mut self, width: Index) -> bool {
        return self.get_widget_properties().set_width(width);
    }

    fn set_height(&mut self, height: Index) -> bool {
        return self.get_widget_properties().set_height(height);
    }

    fn set_dirty(&mut self, is_dirty: bool) {
        return self.get_widget_properties().set_dirty(is_dirty);
    }

    fn set_color_scheme(&mut self, colors: Rc<Scheme>) {
        return self.get_widget_properties().set_color_scheme(colors);
    }

    fn is_dirty(&self) -> bool {
        return self.get_widget_properties().is_dirty();
    }

    fn get_position(&self) -> (Index, Index) { // x, y
        return self.get_widget_properties().get_position();
    }

    fn get_size(&self) -> (Index, Index) { // width, height
        return self.get_widget_properties().get_size();
    }

    // -------------------------------------------
}
