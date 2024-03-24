use mibar_core::{
    widget::{SizeConstraints, Element, Widget, Image},
    image::{Resize, png},
    Size, Rect, Context, DrawCtx, LayoutCtx,
    TypedId, Id, StateHandle
};

pub struct Icon {
    pub icon: Option<png::Data>,
    pub resize: Resize
}

#[derive(Default)]
pub struct IconWidget;

pub struct State {
    image: Option<TypedId<Image>>
}

impl Element for Icon {
    type Widget = IconWidget;

    fn make_state(
        self,
        id: Id,
        ctx: &mut Context
    ) -> <Self::Widget as Widget>::State {
        match self.icon {
            Some(data) => {
                let image = ctx.new_child(
                    id,
                    Image::png(data).resize(self.resize)
                );

                State { image: Some(image) }
            }
            None => State { image: None } 
        } 
    }
}

impl Widget for IconWidget {
    type State = State;

    fn layout(
        handle: StateHandle<Self::State>,
        ctx: &mut LayoutCtx,
        bounds: SizeConstraints
    ) -> Size {
        if let Some(image) = ctx.tree[handle].image {
            ctx.layout(image, bounds)
        } else {
            bounds.min
        }
    }

    fn draw(
        handle: StateHandle<Self::State>,
        ctx: &mut DrawCtx,
        _layout: Rect
    ) {
        if let Some(image) = ctx.tree[handle].image {
            ctx.draw(image);
        }
    }
}
