use crate::{
    asset_loader::{AssetSource, AssetDataSource, AssetId, LoadResult},
    renderer::ImageCacheHandle,
    InitCtx, DrawCtx, LayoutCtx, UpdateCtx, Size, Rect, StateHandle

};
use super::{Element, Widget, SizeConstraints};

#[derive(Debug)]
pub struct Image {
    source: AssetSource
}

#[derive(Debug)]
pub struct ImageWidget;

#[derive(Debug)]
pub struct State {
    id: AssetId,
    handle: ImageCacheHandle
}

pub enum Message {
    ChangeSource(AssetSource)
}

impl Image {
    #[inline]
    pub fn png(source: impl Into<AssetDataSource>) -> Self {
        Self { source: AssetSource::Image(source.into()) }
    }
}

impl Element for Image {
    type Widget = ImageWidget;
    type Message = Message;

    fn make_widget(self, ctx: &mut InitCtx) -> (
        Self::Widget,
        <Self::Widget as Widget>::State
    ) {
        let id = AssetId::new(&self.source);
        let mut handle = ctx.ui.image_cache_handle;

        if !handle.increase_ref_count(id) {
            ctx.load_asset(self.source);
        }

        (ImageWidget, State { id, handle })
    }

    fn message(
        handle: StateHandle<<Self::Widget as Widget>::State>,
        ctx: &mut UpdateCtx,
        msg: Self::Message
    ) {
        let state = &mut ctx.tree[handle];

        match msg {
            Message::ChangeSource(source) => {
                let id = AssetId::new(&source);

                if id == state.id {
                    return;
                }

                state.handle.decrease_ref_count(state.id);
                state.id = id;

                if !state.handle.increase_ref_count(id) {
                    ctx.load_asset(source);
                } else {
                    ctx.ui.request_layout();
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

        match state.handle.size(state.id) {
            Some(size) => bounds.constrain(size),
            None => Size::ZERO
        }
    }

    fn task_result(
        handle: StateHandle<Self::State>,
        ctx: &mut UpdateCtx,
        data: Box<dyn std::any::Any>
    ) {
        let result = data.downcast::<LoadResult>().unwrap();
        let state = &mut ctx.tree[handle];

        match *result {
            Ok(image) => {
                state.handle.allocate(state.id, image);
                ctx.ui.request_layout();
            }
            Err(err) => {
                state.handle.decrease_ref_count(state.id);
                eprintln!("Error while loading image: {}", err);
            }
        }
    }

    fn draw(handle: StateHandle<Self::State>, ctx: &mut DrawCtx, layout: Rect) {
        let state = &ctx.tree[handle];

        let Some(size) = state.handle.size(state.id) else {
            return;
        };

        let available_size = layout.size();

        if size.width > available_size.width || size.height > available_size.height {
            ctx.renderer.push_clip(layout);
            ctx.renderer.render_image(state.id, layout);
            ctx.renderer.pop_clip();
        } else {
            ctx.renderer.render_image(state.id, layout);
        }
    }

    fn destroy(mut state: Self::State) {
        state.handle.decrease_ref_count(state.id)
    }
}
