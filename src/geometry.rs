// This code was basically taken from Xilem/Kurbo.
pub trait FloatExt {
    fn expand(&self) -> f32;
}

#[derive(Clone, Copy, Default, PartialEq, Debug)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32
}

#[derive(Clone, Copy, Default, PartialEq, Debug)]
pub struct Circle {
    pub x: f32,
    pub y: f32,
    pub radius: f32
}

#[derive(Clone, Copy, Default, PartialEq, Debug)]
pub struct Size {
    pub width: f32,
    pub height: f32
}

#[derive(Clone, Copy, Default, PartialEq, Debug)]
pub struct Point {
    pub x: f32,
    pub y: f32
}

impl Size {
    pub const ZERO: Size = Size::new(0f32, 0f32);

    #[inline]
    pub const fn new(width: f32, height: f32) -> Self {
        Size { width, height }
    }

    /// Returns a new `Size` with `width` and `height` rounded
    /// away from zero to the nearest integer, unless they are
    /// already an integer.
    #[inline]
    pub fn expand(self) -> Size {
        Size::new(self.width.expand(), self.height.expand())
    }

    #[inline]
    pub fn clamp(self, min: Self, max: Self) -> Self {
        let width = self.width.clamp(min.width, max.width);
        let height = self.height.clamp(min.height, max.height);

        Self { width, height }
    }
}

impl Rect {
    #[inline]
    pub fn set_size(&mut self, size: Size) {
        self.width = size.width;
        self.height = size.height;
    }

    #[inline]
    pub fn size(&self) -> Size {
        Size { width: self.width, height: self.height }
    }

    #[inline]
    pub fn set_origin(&mut self, point: impl Into<Point>) {
        let point = point.into();

        self.x = point.x;
        self.y = point.y;
    }

    #[inline]
    pub fn origin(&self) -> Point {
        Point::new(self.x, self.y)
    }

    #[must_use]
    #[inline]
    pub fn translate(&self, amount: Size) -> Rect {
        Self {
            x: self.x + amount.width,
            y: self.y + amount.height,
            width: self.width,
            height: self.height
        }
    }

    #[must_use]
    #[inline]
    pub fn shrink(&self, amount: f32) -> Self {
        Self {
            x: self.x + amount,
            y: self.y + amount,
            width: self.width - amount,
            height: self.height - amount
        }
    }
}

impl Point {
    pub const ZERO: Self = Self::new(0f32, 0f32);

    #[inline]
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

impl FloatExt for f32 {
    #[inline]
    fn expand(&self) -> f32 {
        self.abs().ceil().copysign(*self)
    }
}

impl From<(f32, f32)> for Point {
    fn from(point: (f32, f32)) -> Self {
        Self { x: point.0, y: point.1 }
    }
}
