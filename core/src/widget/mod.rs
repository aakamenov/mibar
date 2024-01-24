pub mod flex;
pub mod text;
pub mod button;
pub mod image;
pub mod container;
pub mod state;
mod layout;

pub use layout::*; 
pub use flex::{Flex, FlexBuilder};
pub use text::Text;
pub use button::{Button, ButtonState};
pub use image::Image;
pub use container::Container;
pub use state::{AppState, State};

use std::any::{Any, type_name};

use crate::{
    InitCtx, DrawCtx, LayoutCtx, UpdateCtx,
    Event, StateHandle, Size, Rect
};

pub trait Element {
    type Widget: Widget + 'static;
    type Message;

    fn make_widget(self, ctx: &mut InitCtx) -> (
        Self::Widget,
        <Self::Widget as Widget>::State
    );

    fn message(
        _handle: StateHandle<<Self::Widget as Widget>::State>,
        _ctx: &mut UpdateCtx,
        _msg: Self::Message
    ) { }
}

pub trait Widget {
    type State: 'static;

    fn layout(handle: StateHandle<Self::State>, ctx: &mut LayoutCtx, bounds: SizeConstraints) -> Size;
    fn draw(handle: StateHandle<Self::State>, ctx: &mut DrawCtx, layout: Rect);
    fn event(_handle: StateHandle<Self::State>, _ctx: &mut UpdateCtx, _event: &Event) { }
    fn task_result(_handle: StateHandle<Self::State>, _ctx: &mut UpdateCtx, _data: Box<dyn Any>) {
        panic!(
            "{} is executing async tasks but hasn't implemented Widget::task_result",
            type_name::<Self>()
        );
    }
    fn destroy(_state: Self::State) { }
}

pub trait AnyWidget {
    fn layout(&self, ctx: &mut LayoutCtx, bounds: SizeConstraints) -> Size;
    fn draw(&self, ctx: &mut DrawCtx, layout: Rect);
    fn event(&self, _ctx: &mut UpdateCtx, _event: &Event);
    fn task_result(&self, _ctx: &mut UpdateCtx, _data: Box<dyn Any>);
    fn destroy(&self, state: Box<dyn Any + 'static>);
}

// You may wonder why Box<dyn Any + 'static> is used instead of Box<dyn Any> or just
// &mut dyn Any for the state variable. Because the downcast_mut() call used below won't
// succeed otherwise... Looking at the docs for Any (https://doc.rust-lang.org/std/any/trait.Any.html)
// shows us that there are 3 implementations of that method. The one that is implemented on
// dyn Any + 'static has the actual logic for downcasting and the other two are calling the
// implementation of downcast_mut() for dyn Any which, seems to me, somehow erases the
// concrete type. So if we don't set the dyn Any + 'static bound here, the correct call
// for downcasting won't be invoked. I'm not 100% sure if that's what exactly is happening
// here, but either way, does it make any sense and is it intuitive at all? Fuck no -_-

impl<T: Widget> AnyWidget for T {
    #[inline]
    fn layout(&self, ctx: &mut LayoutCtx, bounds: SizeConstraints) -> Size {
        <T as Widget>::layout(StateHandle::new(ctx.active), ctx, bounds)
    }

    #[inline]
    fn draw(&self, ctx: &mut DrawCtx, layout: Rect) {
        <T as Widget>::draw(StateHandle::new(ctx.active), ctx, layout)
    }

    #[inline]
    fn event(&self, ctx: &mut UpdateCtx, event: &Event) {
        <T as Widget>::event(StateHandle::new(ctx.active), ctx, event)
    }

    #[inline]
    fn task_result(&self, ctx: &mut UpdateCtx, data: Box<dyn Any>) {
        <T as Widget>::task_result(StateHandle::new(ctx.active), ctx, data)
    }

    #[inline]
    fn destroy(&self, state: Box<dyn Any + 'static>) {
        <T as Widget>::destroy(*state.downcast().unwrap());
    }
}
