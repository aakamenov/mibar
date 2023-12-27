mod text;
mod image_cache;

use std::{mem, ops::{Deref, DerefMut}};

use tiny_skia::{
    PixmapMut, PathBuilder, FillRule, Transform,
    Paint, Shader, Mask, Stroke, PixmapPaint,
    FilterQuality, BlendMode
};

use crate::{
    geometry::{Rect, Point},
    color::Color,
    draw::{Quad, Circle, BorderRadius, Background, TextInfo},
    asset_loader::AssetId
};

use image_cache::ImageCache;

pub struct Renderer {
    pub(crate) text_renderer: text::Renderer,
    // Box needed because of ImageCacheHandle.
    pub(crate) image_cache: Box<ImageCache>,
    scale_factor: f32,
    mask: Mask,
    commands: Vec<Command>
}

#[derive(Clone, Copy, Debug)]
pub struct ImageCacheHandle(*mut ImageCache);

#[derive(Debug)]
enum Command {
    Draw(Primitive),
    Clip(Rect),
    PopClip
}

#[derive(Debug)]
enum Primitive {
    Quad {
        rect: Rect,
        background: Shader<'static>,
        border_radius: BorderRadius,
        border_width: f32,
        border_color: Shader<'static>
    },
    Circle {
        pos: Point,
        radius: f32,
        background: Shader<'static>,
        border_width: f32,
        border_color: Shader<'static>
    },
    Text {
        key: text::CacheKey,
        color: tiny_skia::Color,
        rect: Rect
    },
    Image {
        id: AssetId,
        rect: Rect
    }
}

impl Deref for ImageCacheHandle {
    type Target = ImageCache;

    #[inline]
    fn deref(&self) -> &Self::Target {
        unsafe { &(*self.0) }
    }
}

impl DerefMut for ImageCacheHandle {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut (*self.0) }
    }
}

impl Renderer {
    #[inline]
    pub(crate) fn new() -> Self {
        Self {
            text_renderer: text::Renderer::new(),
            image_cache: Box::new(ImageCache::new()),
            scale_factor: 1f32,
            mask: Mask::new(1, 1).unwrap(),
            commands: Vec::with_capacity(64)
        }
    }

    #[inline]
    pub fn scale_factor(&mut self) -> f32 {
        self.scale_factor
    }
    
    #[inline]
    pub fn set_scale_factor(&mut self, scale_factor: f32) {
        self.scale_factor = scale_factor;
        self.text_renderer.invalidate();
    }

    #[inline]
    pub fn fill_quad(&mut self, quad: Quad) {
        self.commands.push(
            Command::Draw(Primitive::Quad {
                rect: quad.rect,
                background: quad.style.background.into(),
                border_radius: quad.style.border_radius,
                border_width: quad.style.border_width,
                border_color: quad.style.border_color.into()
            })
        );
    }

    #[inline]
    pub fn fill_circle(&mut self, circle: Circle) {
        self.commands.push(
            Command::Draw(Primitive::Circle {
                pos: circle.pos,
                radius: circle.radius,
                background: circle.background.into(),
                border_width: circle.border_width,
                border_color: circle.border_color.into()
            })
        );
    }

    #[inline]
    pub fn fill_text(&mut self, info: &TextInfo, rect: Rect, color: Color) {
        let key = self.text_renderer.ensure_is_cached(info, rect.size());
        self.commands.push(Command::Draw(
            Primitive::Text { key, color: color.into(), rect }
        ));
    }

    #[inline]
    pub fn push_clip(&mut self, clip: Rect) {
        self.commands.push(Command::Clip(clip));
    }

    #[inline]
    pub fn pop_clip(&mut self) {
        self.commands.push(Command::PopClip);
    }

    #[inline]
    pub(crate) fn image_cache_handle(&mut self) -> ImageCacheHandle {
        ImageCacheHandle(self.image_cache.as_mut())
    }

    #[inline]
    pub(crate) fn render_image(&mut self, id: AssetId, rect: Rect) {
        self.commands.push(Command::Draw(Primitive::Image { id, rect }));
    }

    pub(crate) fn render(&mut self, pixmap: &mut PixmapMut) {
        if self.mask.width() != pixmap.width() ||
            self.mask.height() != pixmap.height()
        {
            self.mask = Mask::new(pixmap.width(), pixmap.height()).unwrap();
        }

        let mut clip_stack = Vec::with_capacity(8);
        let mut has_clip = false;

        let mut mask_path = PathBuilder::new();
        let mut builder = PathBuilder::new();

        let transform = Transform::from_scale(
            self.scale_factor,
            self.scale_factor
        );

        let mut commands = mem::take(&mut self.commands);
        for command in commands.drain(..) {
            match command {
                Command::Draw(primitive) => {
                    let mask = has_clip.then_some(&self.mask);

                    match primitive {
                        Primitive::Quad {
                            rect,
                            background,
                            border_radius,
                            border_width,
                            border_color
                        } => {
                            let radius = border_radius.0;

                            if radius[0] +
                                radius[1] +
                                radius[2] +
                                radius[3] > 0f32
                            {
                                rounded_rect(&mut builder, rect, radius)
                            } else {
                                builder.push_rect(rect.into());
                            }

                            let path = builder.finish().unwrap();

                            let mut paint = Paint::default();
                            paint.anti_alias = true;
                            paint.shader = background;
                    
                            pixmap.fill_path(
                                &path,
                                &paint,
                                FillRule::EvenOdd,
                                transform,
                                mask
                            );

                            if border_width > 0f32 {
                                paint.shader = border_color;

                                let mut stroke = Stroke::default();
                                stroke.width = border_width;

                                pixmap.stroke_path(
                                    &path,
                                    &paint,
                                    &stroke,
                                    transform,
                                    mask
                                )
                            }

                            builder = path.clear();
                        }
                        Primitive::Circle {
                            pos,
                            radius,
                            background,
                            border_width,
                            border_color
                        } => {
                            builder.push_circle(
                                pos.x,
                                pos.y,
                                radius
                            );

                            let path = builder.finish().unwrap();

                            let mut paint = Paint::default();
                            paint.anti_alias = true;
                            paint.shader = background;

                            pixmap.fill_path(
                                &path,
                                &paint,
                                FillRule::EvenOdd,
                                transform,
                                mask
                            );

                            if border_width > 0f32 {
                                paint.shader = border_color;

                                let mut stroke = Stroke::default();
                                stroke.width = border_width;

                                pixmap.stroke_path(
                                    &path,
                                    &paint,
                                    &stroke,
                                    transform,
                                    mask
                                )
                            }

                            builder = path.clear();
                        }
                        Primitive::Text { key, color, rect } => {
                            if let Some(texture) = self.text_renderer.get_texture(
                                key,
                                color,
                                self.scale_factor
                            ) {
                                let paint = PixmapPaint {
                                    opacity: 1f32,
                                    blend_mode: BlendMode::SourceOver,
                                    quality: FilterQuality::Nearest
                                };
                                
                                pixmap.draw_pixmap(
                                    (rect.x * self.scale_factor) as i32,
                                    (rect.y * self.scale_factor) as i32,
                                    texture,
                                    &paint,
                                    // Glyph images are scaled by cosmic-text
                                    Transform::identity(),
                                    mask
                                );
                            }
                        }
                        Primitive::Image { id, rect } => {
                            if let Some(image) = self.image_cache.get(id) {
                                let paint = PixmapPaint {
                                    opacity: 1f32,
                                    blend_mode: BlendMode::SourceOver,
                                    quality: FilterQuality::Nearest
                                };

                                pixmap.draw_pixmap(
                                    rect.x as i32,
                                    rect.y as i32,
                                    image,
                                    &paint,
                                    transform,
                                    mask
                                );
                            }
                        }
                    }
                }
                Command::Clip(rect) => {
                    has_clip = true;
                    clip_stack.push(rect);
                    mask_path = self.adjust_clip_mask(mask_path, rect, transform);
                }
                Command::PopClip => {
                    if let Some(clip) = clip_stack.pop() {
                        mask_path = self.adjust_clip_mask(mask_path, clip, transform);
                    }

                    has_clip = !clip_stack.is_empty();
                }
            }
        }

        // Assign back the buffer in order to reuse the memory.
        self.commands = commands;
        self.text_renderer.trim();
    }

    #[inline]
    fn adjust_clip_mask(
        &mut self,
        mut builder: PathBuilder,
        clip: Rect,
        transform: Transform
    ) -> PathBuilder {
        self.mask.clear();

        builder.push_rect(clip.into());
        let path = builder.finish().unwrap();

        self.mask.fill_path(
            &path,
            FillRule::EvenOdd,
            false,
            transform
        );

        path.clear()
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

impl<'a> Into<Shader<'a>> for Background {
    #[inline]
    fn into(self) -> Shader<'a> {
        match self {
            Background::Color(color) => Shader::SolidColor(color.into()),
            Background::LinearGradient(gradient) => gradient.0
        }
    }
}

impl Into<tiny_skia::Color> for Color {
    #[inline]
    fn into(self) -> tiny_skia::Color {
        // tiny_skia is ABGR
        tiny_skia::Color::from_rgba8(
            self.b,
            self.g,
            self.r,
            self.a
        )
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