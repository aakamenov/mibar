pub mod flex;
pub mod text;
pub mod button;
mod layout;

pub use layout::*; 
pub use flex::{Flex, FlexBuilder};
pub use text::Text;
pub use button::{Button, ButtonState};

use std::any::{Any, type_name};

use crate::{
    geometry::Size,
    ui::{
        InitCtx, DrawCtx, LayoutCtx,
        UpdateCtx, Event
    }
};

pub trait Element {
    type Widget: Widget + 'static;
    type Message;

    fn make_widget(self, ctx: &mut InitCtx) -> (
        Self::Widget,
        <Self::Widget as Widget>::State
    );

    fn message(
        _state: &mut <Self::Widget as Widget>::State,
        _ctx: &mut UpdateCtx,
        _msg: Self::Message
    ) { }
}

pub trait Widget {
    type State: 'static;

    fn layout(state: &mut Self::State, ctx: &mut LayoutCtx, bounds: SizeConstraints) -> Size;
    fn draw(state: &mut Self::State, ctx: &mut DrawCtx);
    fn event(_state: &mut Self::State, _ctx: &mut UpdateCtx, _event: &Event) { }
    fn task_result(_state: &mut Self::State, _ctx: &mut UpdateCtx, _data: Box<dyn Any>) {
        panic!(
            "{} is executing async tasks but hasn't implemented Widget::task_result",
            type_name::<Self>()
        );
    }
    fn destroy(_state: Self::State) { }
}

pub trait AnyWidget {
    fn layout(
        &self,
        state: &mut Box<dyn Any + 'static>,
        ctx: &mut LayoutCtx,
        bounds: SizeConstraints
    ) -> Size;

    fn draw(
        &self,
        state: &mut Box<dyn Any + 'static>,
        ctx: &mut DrawCtx
    );

    fn event(
        &self,
        _state: &mut Box<dyn Any + 'static>,
        _ctx: &mut UpdateCtx,
        _event: &Event
    );

    fn task_result(
        &self,
        _state: &mut Box<dyn Any + 'static>,
        _ctx: &mut UpdateCtx,
        _data: Box<dyn Any>
    );

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
    fn layout(
        &self,
        state: &mut Box<dyn Any + 'static>,
        ctx: &mut LayoutCtx,
        bounds: SizeConstraints
    ) -> Size {
        <T as Widget>::layout(state.downcast_mut().unwrap(), ctx, bounds)
    }

    #[inline]
    fn draw(
        &self,
        state: &mut Box<dyn Any + 'static>,
        ctx: &mut DrawCtx
    ) {
        <T as Widget>::draw(state.downcast_mut().unwrap(), ctx)
    }

    #[inline]
    fn event(
        &self,
        state: &mut Box<dyn Any + 'static>,
        ctx: &mut UpdateCtx,
        event: &Event
    ) {
        <T as Widget>::event(state.downcast_mut().unwrap(), ctx, event)
    }

    #[inline]
    fn task_result(
        &self,
        state: &mut Box<dyn Any + 'static>,
        ctx: &mut UpdateCtx,
        data: Box<dyn Any>
    ) {
        <T as Widget>::task_result(state.downcast_mut().unwrap(), ctx, data)
    }

    fn destroy(&self, state: Box<dyn Any + 'static>) {
        <T as Widget>::destroy(*state.downcast().unwrap());
    }
}
