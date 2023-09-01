use tiny_skia::{SpreadMode, Transform};

use crate::{color::Color, geometry::Point};

#[derive(Clone, PartialEq, Debug)]
pub struct LinearGradient(
    pub(crate) tiny_skia::Shader<'static>
);

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct GradientStop {
    pos: f32,
    color: Color
}

impl GradientStop {
    /// Creates a new gradient point.
    /// `pos` will be clamped to a 0..=1 range.
    #[inline]
    pub fn new(pos: f32, color: Color) -> Self {
        Self { pos, color }
    }
}

impl LinearGradient {
    /// Creates a new linear gradient.
    /// Returns `None` when:
    ///
    /// - `stops` is empty
    /// - `start` == `end`
    #[inline]
    pub fn new(
        start: Point,
        end: Point,
        stops: [Option<GradientStop>; 8]
    ) -> Option<Self> {
        let stops = stops.into_iter()
            .filter_map(|x| x.map(|stop|
                tiny_skia::GradientStop::new(stop.pos, stop.color.into())
            ))
            .collect();

        let gradient = tiny_skia::LinearGradient::new(
            start.into(),
            end.into(),
            stops,
            SpreadMode::default(),
            Transform::identity()
        )?;

        Some(Self(gradient))
    }
}

impl Into<tiny_skia::Point> for Point {
    #[inline]
    fn into(self) -> tiny_skia::Point {
        tiny_skia::Point { x: self.x, y: self.y }
    }
}
