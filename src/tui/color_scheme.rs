use termion::color::AnsiValue;

use std::cmp;

#[derive(Debug)]
pub struct ColorScheme {
    pub fg_base: AnsiValue,
    pub fg_base_l: AnsiValue,
    pub fg_compl: AnsiValue,
    pub fg_compl_l: AnsiValue,
    pub bg_base: AnsiValue,
    pub bg_base_l: AnsiValue,
    pub bg_compl: AnsiValue,
    pub bg_compl_l: AnsiValue,
}

impl ColorScheme {
    pub fn new() -> ColorScheme {
        ColorScheme {
            fg_base: AnsiValue(15),   // 15 = White
            fg_base_l: AnsiValue(7),  // 7 = Light Grey
            fg_compl: AnsiValue(0),   // 0 = Black
            fg_compl_l: AnsiValue(8), // 8 = Dark Gray
            bg_base: AnsiValue(0),
            bg_base_l: AnsiValue(8),
            bg_compl: AnsiValue(15),
            bg_compl_l: AnsiValue(7),
        }
    }

    // Get the highest color value in the schema.
    // For checking if this schema can be used with the current terminal.
    pub fn max_color(&self) -> u8 {
        let mut max = 0u8;
        max = cmp::max(self.fg_base.0, max);
        max = cmp::max(self.fg_base_l.0, max);
        max = cmp::max(self.fg_compl.0, max);
        max = cmp::max(self.fg_compl_l.0, max);
        max = cmp::max(self.bg_base.0, max);
        max = cmp::max(self.bg_base_l.0, max);
        max = cmp::max(self.bg_compl.0, max);
        max = cmp::max(self.bg_compl_l.0, max);
        max
    }
}
