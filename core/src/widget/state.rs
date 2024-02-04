use std::{marker::PhantomData, ops::{Deref, DerefMut}};

use crate::{
    DrawCtx, LayoutCtx, Context,
    Id, Event, Size, Rect, StateHandle
};

use super::{Element, Widget, SizeConstraints};

pub struct AppState<
    T,
    E: Element,
    S: FnOnce(&mut Context) -> T,
    F: FnOnce(&mut T, StateHandle<State<T>>) -> E
> {
    create_state: S,
    create_child: F,
    data: PhantomData<T>
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

impl<
    T,
    E: Element,
    S: FnOnce(&mut Context) -> T,
    F: FnOnce(&mut T, StateHandle<State<T>>) -> E
> AppState<T, E, S, F> {
    #[inline]
    pub fn new(
        create_state: S,
        create_child: F
    ) -> Self {
        Self {
            create_state,
            create_child,
            data: PhantomData
        }
    }
}

impl<
    T: 'static,
    E: Element + 'static,
    S: FnOnce(&mut Context) -> T,
    F: FnOnce(&mut T, StateHandle<State<T>>) -> E
> Element for AppState<T, E, S, F> {
    type Widget = StateWidget<T>;
    type Message = Message<T>;

    fn make_widget(self, id: Id, ctx: &mut Context) -> (
        Self::Widget,
        <Self::Widget as Widget>::State
    ) {
        let mut state = (self.create_state)(ctx);
        let handle = StateHandle::new(id.0);

        let child = ctx.new_child(id, (self.create_child)(&mut state, handle)).into();

        (
            StateWidget { data: PhantomData },
            State { state, child }
        )
    }

    fn message(
        handle: StateHandle<<Self::Widget as Widget>::State>,
        ctx: &mut Context,
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

    fn event(handle: StateHandle<Self::State>, ctx: &mut Context, event: &Event) {
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
