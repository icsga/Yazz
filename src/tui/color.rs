use termion::color::Rgb;

pub struct Scheme {
    pub fg_light: Rgb,
    pub fg_light2: Rgb,
    pub fg_dark: Rgb,
    pub fg_dark2: Rgb,
    pub bg_light: Rgb,
    pub bg_light2: Rgb,
    pub bg_dark: Rgb,
    pub bg_dark2:Rgb,
}

impl Scheme {
    pub fn new() -> Scheme {
        Scheme {
            fg_dark: Rgb(0, 0, 0),
            fg_dark2: Rgb(10, 10, 10),
            fg_light: Rgb(255, 255, 255),
            fg_light2: Rgb(240, 240, 240),
            bg_dark: Rgb(0, 0, 0),
            bg_dark2: Rgb(10, 10, 10),
            bg_light: Rgb(255, 255, 255),
            bg_light2: Rgb(240, 240, 240),
        }
    }
}
