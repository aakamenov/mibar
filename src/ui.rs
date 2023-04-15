use std::mem;

use tiny_skia::{
    PixmapMut, PathBuilder, FillRule, Transform,
    Paint, Color, LinearGradient, Shader
};
use crate::{
    geometry::{Rect, Circle, Size},
    positioner::Positioner,
    widget::{
        Widget,
        size_constraints::SizeConstraints
    },
    theme::Theme
};

pub struct Ui {
    theme: Theme,
    root: Box<dyn Widget>,
    size: Size
}

pub struct DrawCtx<'a> {
    pub theme: &'a Theme,
    pixmap: &'a mut PixmapMut<'a>,
    builder: PathBuilder
}

pub enum Background {
    Color(Color),
    LinearGradient(LinearGradient)
}

impl Ui {
    pub fn new(root: Box<dyn Widget>) -> Self {
        Self {
            root,
            theme: Theme::light(),   
            size: Size::ZERO
        }
    }

    pub fn layout(&mut self, size: Size) {
        self.size = size;
        self.root.layout(SizeConstraints::tight(size));
    }

    pub fn draw<'a: 'b, 'b>(&'a mut self, pixmap: &'b mut PixmapMut<'b>) {
        assert_eq!(pixmap.width() , self.size.width as u32);
        assert_eq!(pixmap.height() , self.size.height as u32);

        pixmap.fill(self.theme.base);

        let mut ctx = DrawCtx {
            theme: &self.theme,
            pixmap,
            builder: PathBuilder::new()
        };

        self.root.draw(&mut ctx, Positioner::new(self.size));
    }
}

impl<'a> DrawCtx<'a> {
    #[inline]
    pub fn fill_circle(&mut self, circle: Circle, bg: impl Into<Background>) {
        self.builder.push_circle(circle.x, circle.y, circle.radius);
        self.draw_path(bg);

    }

    #[inline]
    pub fn fill_rect(&mut self, rect: Rect, bg: impl Into<Background>) {
        self.builder.push_rect(rect.x, rect.y, rect.width, rect.height);
        self.draw_path(bg);
    }

    fn draw_path(&mut self, bg: impl Into<Background>) {
        let builder = mem::take(&mut self.builder);
        let path = builder.finish().expect("invalid bounds");
        let mut paint = Paint::default();
        
        match bg.into() {
            Background::Color(color) => paint.set_color(color),
            Background::LinearGradient(gradient) =>
                paint.shader = Shader::LinearGradient(gradient)
        }

        paint.anti_alias = true;

        self.pixmap.fill_path(
            &path,
            &paint,
            FillRule::Winding,
            Transform::identity(),
            None
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
