use std::{marker::PhantomData, ops::{Deref, DerefMut}};

use crate::{
    geometry::Size,
    ui::{InitCtx, DrawCtx, LayoutCtx, UpdateCtx, TypedId, Event},
};

use super::{Element, Widget, SizeConstraints};

pub struct State<T, E: Element, F: FnOnce(StateHandle<T>) -> E> {
    state: Box<T>,
    create_child: F
}

pub struct StateWidget<T, E: Element> {
    state: PhantomData<T>,
    child: PhantomData<E>
}

pub enum Message<T, E: Element> {
    Mutate(fn(&mut T, &mut UpdateCtx)),
    MutateClosure(Box<dyn FnOnce(&mut T, &mut UpdateCtx)>),
    Child(E::Message)
}

pub struct InternalState<T, E: Element> {
    state: Box<T>,
    child: TypedId<E>
}

// This pointer to the `state` field in InternalState is safe because
// we only pass the handle to child widgets and the parent's lifetime
// is always longer or equal to its children.
#[derive(Clone, Copy, PartialEq)]
pub struct StateHandle<T>(*mut T);

impl<T, E: Element, F: FnOnce(StateHandle<T>) -> E> State<T, E, F> {
    #[inline]
    pub fn new(state: T, create_child: F) -> Self {
        Self {
            state: Box::new(state),
            create_child
        }
    }
}

impl<T: 'static, E: Element + 'static, F: FnOnce(StateHandle<T>) -> E> Element for State<T, E, F> {
    type Widget = StateWidget<T, E>;
    type Message = Message<T, E>;

    fn make_widget(mut self, ctx: &mut InitCtx) -> (
        Self::Widget,
        <Self::Widget as Widget>::State
    ) {
        let handle = StateHandle(self.state.as_mut());

        (
            StateWidget { state: PhantomData, child: PhantomData },
            InternalState {
                state: self.state,
                child: ctx.new_child((self.create_child)(handle))
            }
        )
    }

    fn message(
        state: &mut <Self::Widget as Widget>::State,
        ctx: &mut UpdateCtx,
        msg: Self::Message
    ) {
        match msg {
            Message::Mutate(mutate) => mutate(&mut state.state, ctx),
            Message::MutateClosure(mutate) => mutate(&mut state.state, ctx),
            Message::Child(msg) => ctx.message(&state.child, msg)
        }
    }
}

impl<T: 'static, E: Element + 'static> Widget for StateWidget<T, E> {
    type State = InternalState<T, E>;

    fn layout(
        state: &mut Self::State,
        ctx: &mut LayoutCtx,
        bounds: SizeConstraints
    ) -> Size {
        ctx.layout(&state.child, bounds)
    }

    fn event(state: &mut Self::State, ctx: &mut UpdateCtx, event: &Event) {
        ctx.event(&state.child, event)
    }

    fn draw(state: &mut Self::State, ctx: &mut DrawCtx) {
        ctx.draw(&state.child)
    }
}

impl<T> Deref for StateHandle<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        unsafe { &(*self.0) }
    }
}

impl<T> DerefMut for StateHandle<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut (*self.0) }
    }
}
