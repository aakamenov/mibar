use cosmic_text::{Family, Stretch, Style, Weight};
use crate::widget::{text, flex};

pub struct Theme {
    pub font: Font,
    pub font_size: f32,
    pub text: text::StyleFn,
    pub flex: flex::StyleFn
}

#[derive(Clone, Copy, PartialEq, Hash, Debug)]
pub struct Font {
    pub family: Family<'static>,
    pub stretch: Stretch,
    pub style: Style,
    pub weight: Weight
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
