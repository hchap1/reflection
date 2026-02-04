use iced::{Color, color};

pub struct Colour;
impl Colour {
    pub fn error() -> Color { color!(0xB33930) }
    pub fn warning() -> Color { color!(0xEBAB34) }
    pub fn loading() -> Color { color!(0x696969) }
    pub fn text() -> Color { color!(0xB0B0B0) }
    pub fn background() -> Color { color!(0x29292e) }
    pub fn foreground() -> Color { color!(0x3B3845) }
    pub fn accent() -> Color { color!(0x4C3F75) }
}
