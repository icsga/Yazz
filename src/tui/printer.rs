use termion::color::AnsiValue;

pub type Index = usize;

pub trait Printer {

    // Set foreground and background color.
    // Color stays set until changed again.
    fn set_color(&mut self, fg_color: AnsiValue, bg_color: AnsiValue);

    // Print some text (might not update the screen)
    fn print(&mut self, x: Index, y: Index, text: &str);

    // Update the screen contents
    fn update(&mut self);
}

