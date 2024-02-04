use std::{rc::Rc, cell::RefCell};

use smallvec::SmallVec;

use crate::{
    widget::{Element, Widget},
    Context, RawWidgetId, TypedId,
    StateHandle, EventSource, EventQueue, Id
};

type EventHandlerFn<E> = fn(
    id: RawWidgetId,
    ctx: &mut Context,
    event: &E
);

pub trait EventHandler<E>: Widget {
    fn handle(
        ctx: &mut Context,
        handle: StateHandle<Self::State>,
        event: &E
    );
}

#[derive(Debug)]
pub struct EventEmitter<E> {
    state: Rc<RefCell<State<E>>>
}

pub struct Binding<E> {
    pub(crate) target: EventEmitter<E>,
    handler: EventHandlerFn<E>,
}

#[derive(Debug)]
pub struct Subscriber<E> {
    id: RawWidgetId,
    handler: EventHandlerFn<E>
}

pub struct Event<E> {
    event: E,
    state: Rc<RefCell<State<E>>>,
    callback: Option<Box<dyn FnOnce(&mut Context, E)>>
}

pub struct SubscriberEvent<E> {
    event: E,
    subscriber: Subscriber<E>,
    callback: Option<Box<dyn FnOnce(&mut Context, E)>>
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
    pub fn subscribe<T: Element>(&self, id: &TypedId<T>)
        where T::Widget: EventHandler<E>
    {
        let handler = |id: RawWidgetId, ctx: &mut Context, event: &E| {
            T::Widget::handle(ctx, StateHandle::new(id), event);
        };

        self.add_subscriber(id.raw(), handler);
    }

    #[inline]
    pub fn create_binding<T: Element>(&self) -> Binding<E>
        where T::Widget: EventHandler<E>
    {
        let handler = |id: RawWidgetId, ctx: &mut Context, event: &E| {
            T::Widget::handle(ctx, StateHandle::new(id), event);
        };

        Binding {
            target: self.clone(),
            handler
        }
    }

    #[inline]
    #[must_use = "You should give this to ctx.event_queue.schedule()."]
    pub fn emit(&self, event: E) -> Event<E> {
        Event {
            event,
            state: self.state.clone(),
            callback: None
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

impl<E> Event<E> {
    #[inline]
    #[must_use = "You should give this to ctx.event_queue.schedule()."]
    pub fn and_then(
        mut self,
        callback: impl FnOnce(&mut Context, E) + 'static
    ) -> Self {
        self.callback = Some(Box::new(callback));

        self
    }
}

impl<E> Subscriber<E> {
    #[inline]
    #[must_use = "You should give this to ctx.event_queue.schedule()."]
    pub fn emit(self, event: E) -> SubscriberEvent<E> {
        SubscriberEvent {
            event,
            subscriber: self,
            callback: None
        }
    }
}

impl<E> SubscriberEvent<E> {
    #[inline]
    #[must_use = "You should give this to ctx.event_queue.schedule()."]
    pub fn and_then(
        mut self,
        callback: impl FnOnce(&mut Context, E) + 'static
    ) -> Self {
        self.callback = Some(Box::new(callback));

        self
    }
}

impl<E: 'static> EventSource for Event<E> {
    #[inline]
    fn emit(self, queue: &EventQueue) {
        let Self { event, state, callback } = self;

        // This logic is written in the shittiest way possible but
        // it has to be in order to make Rust happy :)
        queue.action(move |ctx| {
            state.borrow_mut().subscribers.retain(
                |x| ctx.tree.widgets.contains_key(x.id)
            );

            let len = state.borrow().subscribers.len();

            if len == 0 {
                if let Some(callback) = callback {
                    callback(ctx, event);
                }

                return;
            }

            if len == 1 {
                let sub = state.borrow().subscribers[0];
                (sub.handler)(sub.id, ctx, &event);

                if let Some(callback) = callback {
                    callback(ctx, event);
                }

            } else {
                let mut index = 0;

                while index < len - 1 {
                    let sub = state.borrow().subscribers[index];
                    (sub.handler)(sub.id, ctx, &event);

                    index += 1;
                }

                let sub = state.borrow().subscribers[index];
                (sub.handler)(sub.id, ctx, &event);

                if let Some(callback) = callback {
                    callback(ctx, event);
                }
            }
        });
    }
}

impl<E: 'static> EventSource for SubscriberEvent<E> {
    fn emit(self, queue: &EventQueue) {
        let Self { event, subscriber, callback } = self;

        queue.action(move |ctx| {
            (subscriber.handler)(subscriber.id, ctx, &event);

            if let Some(callback) = callback {
                callback(ctx, event);
            }
        });
    }
}

impl<E> Binding<E> {
    #[inline]
    pub fn bind(self, id: impl Into<Id>) {
        let id = id.into().0;
        let handler = self.handler;

        self.target.state.borrow_mut()
            .subscribers
            .push(Subscriber { id, handler });
    }
}

impl<Event> Clone for EventEmitter<Event> {
    fn clone(&self) -> Self {
        Self { state: Rc::clone(&self.state) }
    }
}

impl<Event> Clone for Subscriber<Event> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            handler: self.handler
        }
    }
}

impl<Event> Copy for Subscriber<Event> { }
