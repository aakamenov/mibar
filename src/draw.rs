use crate::{
    color::Color,
    gradient::LinearGradient,
    geometry::{Rect, Point}
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

impl Quad {
    #[inline]
    pub fn new(rect: Rect, background: impl Into<Background>) -> Self {
        Self {
            rect,
            background: background.into(),
            border_radius: BorderRadius::default(),
            border_width: 0f32,
            border_color: Background::Color(Color::TRANSPARENT)
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
            background: background.into(),
            border_radius: border_radius.into(),
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
