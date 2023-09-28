use crate::geometry::Size;

// This code was basically taken from Xilem/Kurbo.

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SizeConstraints {
    pub min: Size,
    pub max: Size
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
}
