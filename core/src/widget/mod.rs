pub mod flex;
pub mod text;
pub mod button;
pub mod image;
pub mod container;
pub mod state;
mod layout;
mod element_tuple;

use std::rc::Rc;

pub use layout::*; 
pub use flex::Flex;
pub use text::Text;
pub use button::{Button, ButtonState};
pub use image::Image;
pub use container::Container;
pub use state::{AppState, State};
pub use element_tuple::FlexElementTuple;

use std::any::{Any, type_name};

use crate::{
    Context, DrawCtx, Event, Id, TypedId,
    LayoutCtx, Rect, Size, StateHandle,
    WidgetState
};

pub trait Element: Sized {
    type Widget: Widget + 'static;

    fn make_state(self, id: Id, ctx: &mut Context) -> <Self::Widget as Widget>::State;

    fn make(self, ctx: &mut Context) -> TypedId<Self> {
        let id = ctx.tree.widgets.insert(
            WidgetState::new(Rc::new(Self::Widget::default()), Box::new(()))
        );

        let state = Self::make_state(self, Id(id), ctx);
        ctx.tree.widgets[id].state = Box::new(state);

        TypedId::new(id)
    }
}

pub trait Widget: Default {
    type State: 'static;

    fn layout(handle: StateHandle<Self::State>, ctx: &mut LayoutCtx, bounds: SizeConstraints) -> Size;
    fn draw(handle: StateHandle<Self::State>, ctx: &mut DrawCtx, layout: Rect);
    fn event(_handle: StateHandle<Self::State>, _ctx: &mut Context, _event: &Event) { }
    fn task_result(_handle: StateHandle<Self::State>, _ctx: &mut Context, _data: Box<dyn Any>) {
        panic!(
            "{} is executing async tasks but hasn't implemented Widget::task_result",
            type_name::<Self>()
        );
    }
    fn destroy(_state: Self::State) { }
}

pub trait AnyWidget {
    fn layout(&self, id: Id, ctx: &mut LayoutCtx, bounds: SizeConstraints) -> Size;
    fn draw(&self, id: Id, ctx: &mut DrawCtx, layout: Rect);
    fn event(&self, id: Id, _ctx: &mut Context, _event: &Event);
    fn task_result(&self, id: Id, _ctx: &mut Context, _data: Box<dyn Any>);
    fn destroy(&self, state: Box<dyn Any + 'static>);
}

impl<T: Widget> AnyWidget for T {
    #[inline]
    fn layout(&self, id: Id, ctx: &mut LayoutCtx, bounds: SizeConstraints) -> Size {
        <T as Widget>::layout(StateHandle::new(id.0), ctx, bounds)
    }

    #[inline]
    fn draw(&self, id: Id, ctx: &mut DrawCtx, layout: Rect) {
        <T as Widget>::draw(StateHandle::new(id.0), ctx, layout)
    }

    #[inline]
    fn event(&self, id: Id, ctx: &mut Context, event: &Event) {
        <T as Widget>::event(StateHandle::new(id.0), ctx, event)
    }

    #[inline]
    fn task_result(&self, id: Id, ctx: &mut Context, data: Box<dyn Any>) {
        <T as Widget>::task_result(StateHandle::new(id.0), ctx, data)
    }

    #[inline]
    fn destroy(&self, state: Box<dyn Any + 'static>) {
        <T as Widget>::destroy(*state.downcast().unwrap());
    }
}
