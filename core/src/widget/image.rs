use crate::{
    image::{self, Pixmap, Job, Resize, Request, png},
    DrawCtx, LayoutCtx, Context, Size, Rect,
    StateHandle, Id, TypedId
};

#[cfg(feature = "svg")]
use crate::image::svg;

use super::{Element, Widget, SizeConstraints};

#[derive(Clone, Debug)]
pub enum Type {
    Png(png::Data),
    #[cfg(feature = "svg")]
    Svg(svg::Data),
    Pixmap(Pixmap)
}

#[derive(Debug)]
pub struct Image {
    ty: Type,
    resize: Option<Resize>
}

#[derive(Debug, Default)]
pub struct ImageWidget;

#[derive(Debug)]
pub struct State {
    pixmap: Option<Pixmap>,
    #[cfg(feature = "svg")]
    svg: Option<(svg::Data, Option<Resize>)>
}

impl Image {
    #[inline]
    pub fn png(data: impl Into<png::Data>) -> Self {
        Self {
            ty: Type::Png(data.into()),
            resize: None
        }
    }

    #[inline]
    #[cfg(feature = "svg")]
    pub fn svg(data: impl Into<svg::Data>) -> Self {
        Self {
            ty: Type::Svg(data.into()),
            resize: None
        }
    }

    #[inline]
    pub fn rgba(pixels: Vec<u8>, width: u32, height: u32) -> Self {
        Self {
            resize: None,
            ty: Type::Png(png::Data::Rgba {
                width,
                height,
                pixels
            })
        }
    }

    #[inline]
    pub fn from_pixmap(pixmap: Pixmap) -> Self {
        Self {
            ty: Type::Pixmap(pixmap),
            resize: None            
        }
    }

    #[inline]
    pub fn resize(mut self, resize: Resize) -> Self {
        self.resize = Some(resize);

        self
    }
}

impl Element for Image {
    type Widget = ImageWidget;

    fn make_state(self, widget_id: Id, ctx: &mut Context) -> <Self::Widget as Widget>::State {
        match self.ty {
            Type::Png(data) => {
                let sender = ctx.ui.value_sender(widget_id);
                image::load(Job {
                    sender,
                    request: Request::Png(data),
                    resize: self.resize
                });

                #[cfg(feature = "svg")]
                return State { pixmap: None, svg: None };

                #[cfg(not(feature = "svg"))]
                return State { pixmap: None };
            }
            #[cfg(feature = "svg")]
            Type::Svg(data) => {
                let sender = ctx.ui.value_sender(widget_id);
                image::load(Job {
                    sender,
                    request: Request::Svg {
                        data: data.clone(),
                        scale: ctx.ui.scale_factor()
                    },
                    resize: self.resize
                });

                State { pixmap: None, svg: Some((data, self.resize)) }
            }
            Type::Pixmap(pixmap) => {
                let pixmap = if let Some(resize) = self.resize {
                    pixmap.resize(resize)
                } else {
                    Some(pixmap)
                };

                #[cfg(feature = "svg")]
                return State { pixmap, svg: None };

                #[cfg(not(feature = "svg"))]
                return State { pixmap };
            }
        }
    }
}

impl Widget for ImageWidget {
    type State = State;

    fn layout(
        handle: StateHandle<Self::State>,
        ctx: &mut LayoutCtx,
        bounds: SizeConstraints
    ) -> Size {
        let state = &ctx.tree[handle];

        match &state.pixmap {
            Some(pixmap) => {
                let size = pixmap.logical_size();
                let size = Size::new(size.0 as f32, size.1 as f32);

                bounds.constrain(size)
            }
            None => bounds.min
        }
    }

    fn task_result(
        handle: StateHandle<Self::State>,
        ctx: &mut Context,
        data: Box<dyn std::any::Any>
    ) {
        let result = data.downcast::<Result<Pixmap, image::Error>>().unwrap();

        match *result {
            Ok(pixmap) => {
                ctx.tree[handle].pixmap = Some(pixmap);
                ctx.ui.request_layout();
            }
            Err(err) => eprintln!("Error while loading image: {}", err)
        }
    }

    #[cfg(feature = "svg")]
    fn event(handle: StateHandle<Self::State>, ctx: &mut Context, event: &crate::Event) {
        let crate::Event::ScaleFactorChanged(scale_factor) = event else {
            return;
        };

        let state = &mut ctx.tree[handle];
        if let Some((data, resize)) = state.svg.clone() {
            let sender = ctx.ui.value_sender(handle);
            image::load(Job {
                sender,
                request: Request::Svg {
                    data,
                    scale: *scale_factor
                },
                resize
            });
        }
    }

    fn draw(handle: StateHandle<Self::State>, ctx: &mut DrawCtx, layout: Rect) {
        let state = &ctx.tree[handle];

        let Some(pixmap) = state.pixmap.clone() else {
            return;
        };

        let available_size = layout.size();
        let size = pixmap.logical_size();
        let size = Size::new(size.0 as f32, size.1 as f32);

        if size.width > available_size.width || size.height > available_size.height {
            ctx.renderer.push_clip(layout);
            ctx.renderer.render_image(pixmap, layout);
            ctx.renderer.pop_clip();
        } else {
            ctx.renderer.render_image(pixmap, layout);
        }
    }
}

impl TypedId<Image> {
    pub fn change_image(
        self,
        ctx: &mut Context,
        image: impl Into<Type>,
        resize: Option<Resize>
    ) {
        let image = image.into();
        let state = &mut ctx.tree[self];

        match image {
            Type::Pixmap(pixmap) => {
                state.pixmap = Some(pixmap);
                ctx.ui.request_layout();
            }
            Type::Png(data) => {
                let sender = ctx.ui.value_sender(self);
                image::load(Job {
                    sender,
                    request: Request::Png(data),
                    resize
                });
            }
            #[cfg(feature = "svg")]
            Type::Svg(data) => {
                state.svg = Some((data.clone(), resize));

                let sender = ctx.ui.value_sender(self);
                image::load(Job {
                    sender,
                    request: Request::Svg {
                        data,
                        scale: ctx.ui.scale_factor()
                    },
                    resize
                });
            }
        }
    }
}

impl From<png::Data> for Type {
    #[inline]
    fn from(value: png::Data) -> Self {
        Self::Png(value)
    }
}

#[cfg(feature = "svg")]
impl From<svg::Data> for Type {
    #[inline]
    fn from(value: svg::Data) -> Self {
        Self::Svg(value)
    }
}

impl From<Pixmap> for Type {
    #[inline]
    fn from(value: Pixmap) -> Self {
        Self::Pixmap(value)
    }
}
