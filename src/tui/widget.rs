pub type Index = u16;

pub trait Widget {
    fn set_position(&mut self, x: Index, y: Index) -> bool;
    fn set_width(&mut self, width: Index) -> bool;
    fn set_height(&mut self, height: Index) -> bool;
    fn set_dirty(&mut self, is_dirty: bool);
    fn is_dirty(&self) -> bool;
    fn get_position(&self) -> (Index, Index); // x, y
    fn get_size(&self) -> (Index, Index); // width, height
    fn draw(&self);
}
