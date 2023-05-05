use std::mem;

use tiny_skia::{
    PixmapMut, PathBuilder, FillRule, Transform,
    Paint, Color, LinearGradient, Shader, ClipMask
};

use crate::geometry::Rect;

pub struct Renderer<'a> {
    pub(crate) pixmap: &'a mut PixmapMut<'a>,
    pub(crate) builder: PathBuilder,
    pub(crate) clip_stack: Vec<Rect>
}

pub enum Background {
    Color(Color),
    LinearGradient(LinearGradient)
}

impl<'a> Renderer<'a> {
    pub(crate) fn draw_path(&mut self, bg: impl Into<Background>) {
        let builder = mem::take(&mut self.builder);
        let path = builder.finish().expect("invalid bounds");
        let mut paint = Paint::default();
        
        match bg.into() {
            Background::Color(color) => paint.set_color(color),
            Background::LinearGradient(gradient) =>
                paint.shader = Shader::LinearGradient(gradient)
        }

        paint.anti_alias = true;

        let clip_mask = if let Some(rect) = self.clip_stack.last().copied() {
            let mut mask = ClipMask::new();
            let mut path = PathBuilder::new();
            path.push_rect(rect.x, rect.y, rect.width, rect.height);
            mask.set_path(
                rect.width as u32,
                rect.height as u32,
                &path.finish().unwrap(),
                FillRule::EvenOdd,
                false
            );

            Some(mask)
        } else {
            None
        };

        self.pixmap.fill_path(
            &path,
            &paint,
            FillRule::Winding,
            Transform::identity(),
            clip_mask.as_ref()
        );

        self.builder = path.clear();
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
