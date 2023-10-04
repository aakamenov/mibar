use crate::{geometry::{Size, Rect}, FloatExt};

// SizeConstraints implementation was basically taken from Xilem/Kurbo.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct SizeConstraints {
    pub min: Size,
    pub max: Size
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Padding {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Length {
    /// Occupy the minimum amount of space required to fit the child widget.
    Fit,
    /// Occupy the maximum amount of space possible i.e [SizeConstraints.max].
    Expand,
    /// A fixed size to occupy if possible, depending on [SizeConstraints].
    Fixed(f32)
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Alignment {
    Start,
    Center,
    End
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Axis {
    Horizontal,
    Vertical
}

impl SizeConstraints {
    pub const UNBOUNDED: Self = Self {
        min: Size::ZERO,
        max: Size::new(f32::INFINITY, f32::INFINITY)
    };

    #[inline]
    pub fn new(min: Size, max: Size) -> Self {
        Self {
            min: min.expand(),
            max: max.expand()
        }
    }

    #[inline]
    pub fn tight(size: Size) -> Self {
        let size = size.expand();

        Self {
            min: size,
            max: size
        }
    }

    pub fn shrink(&self, diff: Size) -> Self {
        let diff = diff.expand();
        let min = Size::new(
            (self.min.width - diff.width).max(0f32),
            (self.min.height - diff.height).max(0f32),
        );
        let max = Size::new(
            (self.max.width - diff.width).max(0f32),
            (self.max.height - diff.height).max(0f32),
        );

        Self::new(min, max)
    }

    #[inline]
    pub fn loosen(&self) -> Self {
        Self {
            min: Size::ZERO,
            max: self.max,
        }
    }

    #[inline]
    pub fn constrain(&self, size: Size) -> Size {
        size.expand().clamp(self.min, self.max)
    }

    #[inline]
    pub fn pad(&self, padding: Padding) -> Self {
        self.shrink(
            Size::new(padding.horizontal(), padding.vertical())
        )
    }

    pub fn width(mut self, length: impl Into<Length>) -> Self {
        match length.into() {
            Length::Fit => { }
            Length::Expand => {
                self.min.width = self.max.width;
            }
            Length::Fixed(size) => {
                let width = size.expand().min(self.max.width).max(self.min.width);
                self.min.width = width;
                self.max.width = width;
            }
        }

        self
    }

    pub fn height(mut self, length: impl Into<Length>) -> Self {
        match length.into() {
            Length::Fit => { }
            Length::Expand => {
                self.min.height = self.max.height;
            }
            Length::Fixed(size) => {
                let height = size.expand().min(self.max.height).max(self.min.height);
                self.min.height = height;
                self.max.height = height;
            }
        }

        self
    }
}

impl Padding {
    pub const ZERO: Self = Self::new(0f32, 0f32, 0f32, 0f32);

    #[inline]
    pub const fn new(top: f32, right: f32, bottom: f32, left: f32) -> Self {
        Self { top, right, bottom, left }
    }

    #[inline]
    pub fn vertical(&self) -> f32 {
        self.top + self.bottom
    }

    #[inline]
    pub fn horizontal(&self) -> f32 {
        self.right + self.left
    } 
}

impl Alignment {
    pub fn align(&self, rect: &mut Rect, space: f32, axis: Axis) {
        let (value, size) = match axis {
            Axis::Horizontal => (&mut rect.x, rect.width),
            Axis::Vertical => (&mut rect.y, rect.height)
        };

        match self {
            Self::Start => { }
            Self::Center => *value += (space - size) / 2f32,
            Self::End => *value += space - size
        }
    }
}

impl Axis {
    #[inline]
    pub fn flip(&self) -> Self {
        match self {
            Self::Horizontal => Self::Vertical,
            Self::Vertical => Self::Horizontal
        }
    }

    #[inline]
    pub fn main(&self, size: Size) -> f32 {
        match self {
            Self::Horizontal => size.width,
            Self::Vertical => size.height
        }
    }

    #[inline]
    pub fn cross(&self, size: Size) -> f32 {
        match self {
            Self::Horizontal => size.height,
            Self::Vertical => size.width
        }
    }

    #[inline]
    pub fn main_and_cross_size(&self, size: Size) -> (f32, f32) {
        match self {
            Self::Horizontal => (size.width, size.height),
            Self::Vertical => (size.height, size.width)
        }
    }

    #[inline]
    pub fn main_and_cross(&self, main: f32, cross: f32) -> (f32, f32) {
        match self {
            Self::Horizontal => (main, cross),
            Self::Vertical => (cross, main)
        }
    }
}

impl From<f32> for Padding {
    #[inline]
    fn from(value: f32) -> Self {
        Padding::new(value, value, value, value)
    }
}

impl From<f32> for Length {
    #[inline]
    fn from(value: f32) -> Self {
        Length::Fixed(value)
    }
} 
