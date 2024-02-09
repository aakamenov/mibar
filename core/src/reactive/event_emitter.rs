use std::{rc::Rc, cell::RefCell};

use smallvec::SmallVec;

use crate::{
    widget::{Element, Widget},
    Context, RawWidgetId, TypedId, StateHandle, Id
};

type EventHandlerFn<E> = fn(
    id: RawWidgetId,
    ctx: &mut Context,
    event: &E
);

pub trait EventHandler<E>: Widget {
    fn handle(
        handle: StateHandle<Self::State>,
        ctx: &mut Context,
        event: &E
    );
}

#[derive(Debug)]
pub struct EventEmitter<E> {
    state: Rc<RefCell<State<E>>>
}

#[derive(Debug)]
pub struct Subscriber<E> {
    id: RawWidgetId,
    handler: EventHandlerFn<E>
}

#[derive(Debug)]
struct State<E> {
    subscribers: SmallVec<[Subscriber<E>; 8]>
}

impl<E> EventEmitter<E> {
    #[inline]
    pub fn new() -> Self {
        Self {
            state: Rc::new(
                RefCell::new(
                    State { subscribers: SmallVec::new() }
                )
            )
        }
    }

    #[inline]
    pub fn subscribe<T: Element>(&self, id: TypedId<T>)
        where T::Widget: EventHandler<E>
    {
        let handler = |id: RawWidgetId, ctx: &mut Context, event: &E| {
            T::Widget::handle(StateHandle::new(id), ctx, event);
        };

        self.add_subscriber(id.raw(), handler);
    }

    #[inline]
    pub fn emit(&self, ctx: &mut Context, event: &E) {
        self.state.borrow_mut().subscribers.retain(
            |x| ctx.tree.widgets.contains_key(x.id)
        );

        let len = self.state.borrow().subscribers.len();

        for i in 0..len {
            let sub = self.state.borrow().subscribers[i];
            sub.call(ctx, event);
        }
    }

    pub fn subscriber(&self, id: Id) -> Option<Subscriber<E>> {
        self.state.borrow()
            .subscribers
            .iter()
            .find(|x| x.id == id.0).copied()
    }

    #[inline]
    pub(crate) fn last_added(&self) -> Option<Subscriber<E>> {
        self.state.borrow().subscribers.last().copied()
    }

    #[inline]
    fn add_subscriber(&self, id: RawWidgetId, handler: EventHandlerFn<E>) {
        // We deliberately don't check if the id already exists as that scenario is
        // very unlikely and should be considered a programmer error if it happened.
        self.state.borrow_mut().subscribers.push(Subscriber { id, handler });
    }
}

impl<E> Subscriber<E> {
    #[inline]
    pub fn call(&self, ctx: &mut Context, event: &E) {
        (self.handler)(self.id, ctx, event);
    }
}

impl<E> Clone for EventEmitter<E> {
    fn clone(&self) -> Self {
        Self { state: Rc::clone(&self.state) }
    }
}

impl<E> Clone for Subscriber<E> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            handler: self.handler
        }
    }
}

impl<E> Copy for Subscriber<E> { }
