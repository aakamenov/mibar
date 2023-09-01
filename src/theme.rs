use cosmic_text::{Family, Stretch, Style, Weight};

use crate::color::Color;

pub struct Theme {
    pub font: Font,
    pub font_size: f32,
    pub base: Color,
    pub surface: Color,
    pub overlay: Color,
    pub muted: Color,
    pub subtle: Color,
    pub text: Color,
    pub warm1: Color,
    pub warm2: Color,
    pub warm3: Color,
    pub cold1: Color,
    pub cold2: Color,
    pub cold3: Color
}

#[derive(Clone, Copy, PartialEq, Hash, Debug)]
pub struct Font {
    pub family: Family<'static>,
    pub stretch: Stretch,
    pub style: Style,
    pub weight: Weight
}

impl Theme {
    #[inline]
    pub fn light() -> Self {
        Self {
            font: Font {
                family: Family::Name("SauceCodePro Nerd Font"),
                ..Font::default()
            },
            font_size: 16f32,
            base: Color::rgb(250, 244, 237),
            surface: Color::rgb(255, 250, 243),
            overlay: Color::rgb(242, 233, 222),
            muted: Color::rgb(152, 147, 165),
            subtle: Color::rgb(121, 117, 147),
            text: Color::rgb(87, 82, 121),
            warm1: Color::rgb(180, 99, 122),
            warm2: Color::rgb(234, 157, 52),
            warm3: Color::rgb(215, 130, 126),
            cold1: Color::rgb(40, 105, 131),
            cold2: Color::rgb(86, 148, 159),
            cold3: Color::rgb(144, 122, 169)
        }
    }
}

impl Default for Font {
    #[inline]
    fn default() -> Self {
        Self {
            family: Family::SansSerif,
            stretch: Stretch::Normal,
            style: Style::Normal,
            weight: Weight::NORMAL
        }
    }
}
