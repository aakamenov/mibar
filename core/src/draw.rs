use std::{mem, hash::{Hash, Hasher}};

use crate::{
    color::Color,
    gradient::LinearGradient,
    geometry::{Rect, Point},
    theme::Font
};

#[derive(Clone, PartialEq, Debug)]
pub enum Background {
    Color(Color),
    LinearGradient(LinearGradient)
}

#[derive(Default, Clone, Copy, PartialEq, PartialOrd, Debug)]
pub struct BorderRadius(pub [f32; 4]);

#[derive(Clone, Debug)]
pub struct Quad {
    pub rect: Rect,
    pub style: QuadStyle
}

#[derive(Clone, PartialEq, Debug)]
pub struct QuadStyle {
    pub background: Background,
    pub border_radius: BorderRadius,
    pub border_width: f32,
    pub border_color: Background
}

#[derive(Clone, Debug)]
pub struct Circle {
    pub pos: Point,
    pub radius: f32,
    pub background: Background,
    pub border_width: f32,
    pub border_color: Background
}

#[derive(Clone, Debug)]
pub struct TextInfo<'a> {
    pub text: &'a str,
    pub size: f32,
    pub line_height: LineHeight,
    pub font: Font
}

// We use the same approach as Iced here. The rationale being
// that multiplying the text size by 1.2 will work for most fonts.
// So this is what is used as a default (LineHeight::Relative(1.2)).
// Apparently this is what web browsers tend to do as well.
// Otherwise, it can be set by the user if needed.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum LineHeight {
    /// A scale that the size of the text is multiplied by.
    Relative(f32),
    /// An absolute height in logical pixels.
    Absolute(f32)
}

impl Quad {
    #[inline]
    pub fn new(rect: Rect, background: impl Into<Background>) -> Self {
        Self {
            rect,
            style: QuadStyle {
                background: background.into(),
                border_radius: BorderRadius::default(),
                border_width: 0f32,
                border_color: Background::Color(Color::TRANSPARENT)
            }
        }
    }

    #[inline]
    pub fn rounded(
        rect: Rect,
        background: impl Into<Background>,
        border_radius: impl Into<BorderRadius>
    ) -> Self {
        Self {
            rect,
            style: QuadStyle {
                background: background.into(),
                border_radius: border_radius.into(),
                border_width: 0f32,
                border_color: Background::Color(Color::TRANSPARENT)
            }
        }
    }

    #[inline]
    pub fn with_border(
        mut self,
        width: f32,
        color: impl Into<Background>
    ) -> Self {
        self.style.border_width = width;
        self.style.border_color = color.into();

        self
    }
}

impl QuadStyle {
    #[inline]
    pub fn solid_background(background: Color) -> Self {
        Self {
            background: Background::Color(background),
            border_radius: 0f32.into(),
            border_width: 0f32,
            border_color: Background::Color(Color::TRANSPARENT)
        }
    }

    #[inline]
    pub fn gradient(gradient: LinearGradient) -> Self {
        Self {
            background: Background::LinearGradient(gradient),
            border_radius: 0f32.into(),
            border_width: 0f32,
            border_color: Background::Color(Color::TRANSPARENT)
        }
    }

    #[inline]
    pub fn rounded(
        mut self,
        border_radius: impl Into<BorderRadius>
    ) -> Self {
        self.border_radius = border_radius.into();

        self
    }

    #[inline]
    pub fn with_border(
        mut self,
        width: f32,
        color: impl Into<Background>
    ) -> Self {
        self.border_width = width;
        self.border_color = color.into();

        self
    }
}

impl Circle {
    #[inline]
    pub fn new(
        pos: impl Into<Point>,
        radius: f32,
        background: impl Into<Background>
    ) -> Self {
        Self {
            pos: pos.into(),
            radius,
            background: background.into(),
            border_width: 0f32,
            border_color: Background::Color(Color::TRANSPARENT)
        }
    }

    #[inline]
    pub fn with_border(
        mut self,
        width: f32,
        color: impl Into<Background>
    ) -> Self {
        self.border_width = width;
        self.border_color = color.into();

        self
    }
}

impl<'a> TextInfo<'a> {
    #[inline]
    pub fn new(text: &'a str, size: f32) -> Self {
        Self {
            text,
            size,
            line_height: LineHeight::default(),
            font: Font::default()
        }
    }

    #[inline]
    pub fn with_font(mut self, font: Font) -> Self {
        self.font = font;

        self
    }

    #[inline]
    pub fn with_line_height(mut self, height: LineHeight) -> Self {
        self.line_height = height;

        self
    }
}

impl From<Color> for Background {
    #[inline]
    fn from(value: Color) -> Self {
        Self::Color(value)
    }
}

impl From<LinearGradient> for Background {
    #[inline]
    fn from(value: LinearGradient) -> Self {
        Self::LinearGradient(value)
    }
}

impl From<[f32; 4]> for BorderRadius {
    #[inline]
    fn from(value: [f32; 4]) -> Self {
        Self(value)
    }
}

impl From<f32> for BorderRadius {
    #[inline]
    fn from(value: f32) -> Self {
        Self([value, value, value, value])
    }
}

impl LineHeight {
    #[inline]
    pub fn to_absolute(self, text_size: f32) -> f32 {
        match self {
            Self::Relative(scale) => scale * text_size,
            Self::Absolute(height) => height
        }
    }
}

impl Hash for LineHeight {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        let variant = mem::discriminant(self);
        
        match self {
            LineHeight::Relative(scale) => 
                (variant, scale.to_bits()).hash(state),
            LineHeight::Absolute(height) =>
                (variant, height.to_bits()).hash(state)
        }
    }
}

impl Default for LineHeight {
    fn default() -> Self {
        Self::Relative(1.2)
    }
}
