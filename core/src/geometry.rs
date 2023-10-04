use std::ops::{Add, AddAssign, Sub, SubAssign};

use crate::widget::Padding;

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
pub struct Size {
    pub width: f32,
    pub height: f32
}

#[derive(Clone, Copy, Default, PartialEq, Debug)]
pub struct Point {
    pub x: f32,
    pub y: f32
}

#[derive(Clone, Copy, Default, PartialEq, Debug)]
pub struct Vector {
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
    pub const fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self { x, y, width, height }
    }

    #[inline]
    pub const fn from_size(size: Size) -> Self {
        Self::new(0f32, 0f32, size.width, size.height)
    }

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
    pub fn translate(&self, amount: Vector) -> Rect {
        Self {
            x: self.x + amount.x,
            y: self.y + amount.y,
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
            width: self.width - amount - amount,
            height: self.height - amount - amount
        }
    }

    #[inline]
    pub fn contains(&self, point: impl Into<Point>) -> bool {
        let point = point.into();

        point.x >= self.x &&
            point.x < self.x + self.width &&
            point.y >= self.y &&
            point.y < self.y + self.height
    }

    #[must_use]
    #[inline]
    pub fn intersect(&self, other: Self) -> Option<Self> {
        let x = self.x.max(other.x);
        let y = self.y.max(other.y);
        let width = (self.x + self.width).min(other.x + other.width);
        let height = (self.y + self.height).min(other.y + other.height);

        if width > x && height > y {
            Some(Self {
                x,
                y,
                width: width - x,
                height: height - y
            })
        } else {
            None
        }
    }

    #[inline]
    pub fn overlaps(&self, other: Self) -> bool {
        other.x + other.width  >= self.x && other.x <= self.x + self.width &&
            other.y + other.height >= self.y && other.y <= self.y + self.height
    }

    #[inline]
    pub fn center(&self) -> Point {
        Point::new(
            self.x + (self.width / 2f32),
            self.y + (self.height / 2f32),
        )
    }
}

impl Point {
    pub const ZERO: Self = Self::new(0f32, 0f32);

    #[inline]
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

impl Vector {
    #[inline]
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

impl From<Padding> for Size {
    #[inline]
    fn from(value: Padding) -> Self {
        Self::new(value.horizontal(), value.vertical())
    }
}

impl Add for Point {
    type Output = Self;

    #[inline]
    fn add(self, rhs: Self) -> Self::Output {
        Self::new(self.x + rhs.x, self.y + rhs.y)
    }
}

impl AddAssign for Point {
    #[inline]
    fn add_assign(&mut self, rhs: Self) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}

impl Sub for Point {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: Self) -> Self::Output {
        Self::new(self.x - rhs.x, self.y - rhs.y)
    }
}

impl SubAssign for Point {
    #[inline]
    fn sub_assign(&mut self, rhs: Self) {
        self.x -= rhs.x;
        self.y -= rhs.y;
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

impl From<(f32, f32)> for Vector {
    fn from(vector: (f32, f32)) -> Self {
        Self { x: vector.0, y: vector.1 }
    }
}
