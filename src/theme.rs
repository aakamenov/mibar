use tiny_skia::Color;
use cosmic_text::{Family, Stretch, Style, Weight};

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
            font: Font::default(),
            font_size: 16f32,
            base: Color::from_rgba8(250, 244, 237, 255),
            surface: Color::from_rgba8(255, 250, 243, 255),
            overlay: Color::from_rgba8(242, 233, 222, 255),
            muted: Color::from_rgba8(152, 147, 165, 255),
            subtle: Color::from_rgba8(121, 117, 147, 255),
            text: Color::from_rgba8(87, 82, 121, 255),
            warm1: Color::from_rgba8(180, 99, 122, 255),
            warm2: Color::from_rgba8(234, 157, 52, 255),
            warm3: Color::from_rgba8(215, 130, 126, 255),
            cold1: Color::from_rgba8(40, 105, 131, 255),
            cold2: Color::from_rgba8(86, 148, 159, 255),
            cold3: Color::from_rgba8(144, 122, 169, 255)
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
