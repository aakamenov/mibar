use std::{marker::PhantomData, ops::{Deref, DerefMut}};

use crate::{
    InitCtx, DrawCtx, LayoutCtx, UpdateCtx,
    Id, Event, Size, Rect, StateHandle
};

use super::{Element, Widget, SizeConstraints};

pub struct AppState<
    T,
    E: Element,
    F: FnOnce(StateHandle<State<T>>) -> E
> {
    state: T,
    create_child: F
}

pub struct StateWidget<T> {
    data: PhantomData<T>
}

pub enum Message<T> {
    Mutate(fn(&mut T)),
    MutateClosure(Box<dyn FnOnce(&mut T)>)
}

pub struct State<T> {
    pub state: T,
    child: Id
}

impl<T, E: Element, F: FnOnce(StateHandle<State<T>>) -> E> AppState<T, E, F> {
    #[inline]
    pub fn new(state: T, create_child: F) -> Self {
        Self {
            state,
            create_child
        }
    }
}

impl<
    T: 'static,
    E: Element + 'static,
    F: FnOnce(StateHandle<State<T>>) -> E
> Element for AppState<T, E, F> {
    type Widget = StateWidget<T>;
    type Message = Message<T>;

    fn make_widget(self, ctx: &mut InitCtx) -> (
        Self::Widget,
        <Self::Widget as Widget>::State
    ) {
        let handle = StateHandle::new(ctx.active);

        (
            StateWidget { data: PhantomData },
            State {
                state: self.state,
                child: ctx.new_child((self.create_child)(handle)).into()
            }
        )
    }

    fn message(
        handle: StateHandle<<Self::Widget as Widget>::State>,
        ctx: &mut UpdateCtx,
        msg: Self::Message
    ) {
        let state = &mut ctx.tree[handle];
        match msg {
            Message::Mutate(mutate) => mutate(&mut state.state),
            Message::MutateClosure(mutate) => mutate(&mut state.state)
        }
    }
}

impl<T: 'static> Widget for StateWidget<T> {
    type State = State<T>;

    fn layout(
        handle: StateHandle<Self::State>,
        ctx: &mut LayoutCtx,
        bounds: SizeConstraints
    ) -> Size {
        ctx.layout(ctx.tree[handle].child, bounds)
    }

    fn event(handle: StateHandle<Self::State>, ctx: &mut UpdateCtx, event: &Event) {
        ctx.event(ctx.tree[handle].child, event)
    }

    fn draw(handle: StateHandle<Self::State>, ctx: &mut DrawCtx, _layout: Rect) {
        ctx.draw(ctx.tree[handle].child)
    }
}

impl<T> Deref for State<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.state
    }
}

impl<T> DerefMut for State<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.state
    }
}
