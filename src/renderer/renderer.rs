use std::mem;

use tiny_skia::{
    PixmapMut, PathBuilder, FillRule, Transform,
    Paint, Color, LinearGradient, Shader, Mask,
    Stroke
};

use crate::geometry::{Rect, Point};

pub struct Renderer {
    mask: Mask,
    mask_path: PathBuilder,
    builder: PathBuilder
}

pub struct RenderPass<'a: 'b, 'b> {
    pixmap: &'b mut PixmapMut<'a>,
    mask: Mask,
    mask_path: PathBuilder,
    builder: PathBuilder
}

#[derive(Clone, Debug)]
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

impl Renderer {
    #[inline]
    pub fn new() -> Self {
        Self {
            mask: Mask::new(1, 1).unwrap(),
            mask_path: PathBuilder::new(),
            builder: PathBuilder::new()
        }
    }

    pub fn begin<'a: 'b, 'b>(mut self, pixmap: &'b mut PixmapMut<'a>) -> RenderPass<'a, 'b> {
        if self.mask.width() != pixmap.width() ||
            self.mask.height() != pixmap.height()
        {
            self.mask = Mask::new(pixmap.width(), pixmap.height()).unwrap();
        }

        RenderPass {
            pixmap,
            mask: self.mask,
            mask_path: self.mask_path,
            builder: self.builder
        }
    }
}

impl<'a: 'b, 'b> RenderPass<'a, 'b> {
    pub fn end(self) -> Renderer {
        Renderer {
            mask: self.mask,
            mask_path: self.mask_path,
            builder: self.builder
        }
    }

    pub fn draw_quad(&mut self, quad: Quad) {
        let radius = quad.border_radius.0;
        if radius[0] +
            radius[1] +
            radius[2] +
            radius[3] > 0f32
        {
            rounded_rect(&mut self.builder, quad.rect, radius)
        } else {
            self.builder.push_rect(quad.rect.into());
        }

        let path = mem::take(&mut self.builder).finish().unwrap();

        let mut paint = Paint::default();
        paint.anti_alias = true;
        paint.shader = quad.background.into();

        self.pixmap.fill_path(
            &path,
            &paint,
            FillRule::EvenOdd,
            Transform::identity(),
            Some(&self.mask)
        );

        if quad.border_width > 0f32 {
            paint.shader = quad.border_color.into();

            let mut stroke = Stroke::default();
            stroke.width = quad.border_width;

            self.pixmap.stroke_path(
                &path,
                &paint,
                &stroke,
                Transform::identity(),
                Some(&self.mask)
            )
        }

        self.builder = path.clear();
    }

    pub fn draw_circle(&mut self, circle: Circle) {
        self.builder.push_circle(
            circle.pos.x,
            circle.pos.y,
            circle.radius
        );

        let path = mem::take(&mut self.builder).finish().unwrap();

        let mut paint = Paint::default();
        paint.anti_alias = true;
        paint.shader = circle.background.into();

        self.pixmap.fill_path(
            &path,
            &paint,
            FillRule::EvenOdd,
            Transform::identity(),
            Some(&self.mask)
        );

        if circle.border_width > 0f32 {
            paint.shader = circle.border_color.into();

            let mut stroke = Stroke::default();
            stroke.width = circle.border_width;

            self.pixmap.stroke_path(
                &path,
                &paint,
                &stroke,
                Transform::identity(),
                Some(&self.mask)
            )
        }

        self.builder = path.clear();
    }

    pub fn set_clip(&mut self, clip: Rect) {
        self.mask.clear();

        self.mask_path.push_rect(clip.into());
        let path = mem::take(&mut self.mask_path).finish().unwrap();

        self.mask.fill_path(
            &path,
            FillRule::EvenOdd,
            false,
            Transform::identity()
        );

        self.mask_path = path.clear();
    }
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

    #[inline]
    pub fn bounds(&self) -> Rect {
        Rect::new(
            self.pos.x - self.radius,
            self.pos.y - self.radius,
            self.radius,
            self.radius
        )
    }
}

#[inline(always)]
fn rounded_rect(
    builder: &mut PathBuilder,
    rect: Rect,
    radius: [f32; 4]
) {
    let [tl, tr, br, bl] = radius;
    let mut cursor = Point::new(rect.x, rect.y + tl);

    builder.move_to(cursor.x, cursor.y);

    builder.cubic_to(
        cursor.x,
        cursor.y,
        cursor.x,
        cursor.y - tl,
        {
            cursor.x += tl;
            cursor.x
        },
        {
            cursor.y -= tl;
            cursor.y
        }
    );
    
    builder.line_to(
        {
            cursor.x = rect.x + rect.width - tr;
            cursor.x
        },
        cursor.y
    );

    builder.cubic_to(
        cursor.x,
        cursor.y,
        cursor.x + tr,
        cursor.y,
        {
            cursor.x += tr;
            cursor.x
        },
        {
            cursor.y += tr;
            cursor.y
        }
    );

    builder.line_to(
        cursor.x,
        {
            cursor.y = rect.y + rect.height - br;
            cursor.y
        }
    );

    builder.cubic_to(
        cursor.x,
        cursor.y,
        cursor.x,
        cursor.y + br,
        {
            cursor.x -= br;
            cursor.x
        },
        {
            cursor.y += br;
            cursor.y
        }
    );

    builder.line_to(
        {
            cursor.x = rect.x + bl;
            cursor.x
        },
        cursor.y
    );

    builder.cubic_to(
        cursor.x,
        cursor.y,
        cursor.x - bl,
        cursor.y,
        {
            cursor.x -= bl;
            cursor.x
        },
        {
            cursor.y -= bl;
            cursor.y
        }
    );

    builder.close();
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

impl<'a> Into<Shader<'a>> for Background {
    #[inline]
    fn into(self) -> Shader<'a> {
        match self {
            Background::Color(color) => Shader::SolidColor(color),
            Background::LinearGradient(gradient) =>
                Shader::LinearGradient(gradient)
        }
    }
}

impl Into<tiny_skia::Rect> for Rect {
    #[inline]
    fn into(self) -> tiny_skia::Rect {
        tiny_skia::Rect::from_xywh(
            self.x,
            self.y,
            self.width,
            self.height
        ).expect("convert to tiny_skia::Rect")
    }
}
