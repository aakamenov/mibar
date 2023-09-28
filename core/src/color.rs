#[derive(Clone, Copy, PartialEq, Hash, PartialOrd, Debug)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8
}

impl Color {
    pub const TRANSPARENT: Self = Self::rgba(0, 0, 0, 0);
    pub const BLACK: Self = Self::rgb(0, 0, 0);
    pub const WHITE: Self = Self::rgb(255, 255, 255);
    pub const RED: Self = Self::rgb(255, 0, 0);
    pub const GREEN: Self = Self::rgb(0, 255, 0);
    pub const BLUE: Self = Self::rgb(0, 0, 255);

    #[inline]
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255}
    }

    #[inline]
    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a}
    }

    /// Multiplies the color's alpha value by `opacity`.
    /// `opacity` will be clamped to the 0..=1 range first.
    #[inline]
    pub fn apply_opacity(&mut self, opacity: f32) {
        let opacity = 1f32.min(opacity).max(0f32);
        unsafe {
            self.apply_opacity_unchecked(opacity);
        }
    }

    /// Multiplies the color's alpha value by `opacity`.
    /// 
    /// # Safety
    /// `n` must be in 0..=1 range.
    #[inline]
    pub unsafe fn apply_opacity_unchecked(&mut self, opacity: f32) {
        let alpha = (f32::from(self.a) / 255.0) * opacity;
        self.a = (alpha * 255.0).round() as u8;
    }
}
