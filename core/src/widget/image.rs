use crate::{
    image::{self, Pixmap, Data, Job, Resize},
    DrawCtx, LayoutCtx, Context, Size, Rect, StateHandle, Id, TypedId
};
use super::{Element, Widget, SizeConstraints};

#[derive(Clone, Debug)]
pub enum Type {
    Data(Data),
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
    pixmap: Option<Pixmap>
}

impl Image {
    #[inline]
    pub fn png(data: impl Into<Data>) -> Self {
        Self {
            ty: Type::Data(data.into()),
            resize: None
        }
    }

    #[inline]
    pub fn rgba(pixels: Vec<u8>, width: u32, height: u32) -> Self {
        Self {
            resize: None,
            ty: Type::Data(Data::Rgba {
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
            Type::Data(data) => {
                let sender = ctx.ui.value_sender(widget_id);
                image::load(Job { sender, data, resize: self.resize });

                State { pixmap: None }
            }
            Type::Pixmap(pixmap) => {
                State {
                    pixmap: if let Some(resize) = self.resize {
                        pixmap.resize(resize)
                    } else {
                        Some(pixmap)
                    }
                }
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
                let size = Size::new(pixmap.width() as f32, pixmap.height() as f32);

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

    fn draw(handle: StateHandle<Self::State>, ctx: &mut DrawCtx, layout: Rect) {
        let Some(pixmap) = ctx.tree[handle].pixmap.clone() else {
            return;
        };

        let available_size = layout.size();
        let size = Size::new(pixmap.width() as f32, pixmap.height() as f32);

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
        image: impl Into<Type>
    ) {
        let image = image.into();
        let state = &mut ctx.tree[self];

        match image {
            Type::Pixmap(pixmap) => {
                state.pixmap = Some(pixmap);
                ctx.ui.request_layout();
            }
            Type::Data(data) => {
                let sender = ctx.ui.value_sender(self);
                image::load(Job { sender, data, resize: None });
            }
        }
    }
}

impl From<Data> for Type {
    #[inline]
    fn from(value: Data) -> Self {
        Self::Data(value)
    }
}

impl From<Pixmap> for Type {
    #[inline]
    fn from(value: Pixmap) -> Self {
        Self::Pixmap(value)
    }
}
