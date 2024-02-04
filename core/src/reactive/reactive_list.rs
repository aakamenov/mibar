use std::{
    cell::{RefCell, RefMut},
    ops::{Deref, DerefMut},
    rc::Rc,
    fmt
};

use crate::{widget::Element, TypedId, Id, EventQueue};
use super::event_emitter::{self, EventEmitter, EventHandler, Event};

pub struct ReactiveList<T> {
    items: Rc<RefCell<Vec<T>>>,
    emitter: EventEmitter<ListOp<T>>
}

pub enum ListOp<T> {
    Init(ListRef<T>),
    Push(T)
}

pub struct Binding<T> {
    target: ReactiveList<T>,
    inner: event_emitter::Binding<ListOp<T>>
}

pub struct ListSlice<'a, T>(RefMut<'a, Vec<T>>);

pub struct ListRef<T>(Rc<RefCell<Vec<T>>>);

impl<T> ReactiveList<T> {
    #[inline]
    pub fn new() -> Self {
        Self::from_vec(Vec::new())
    }

    #[inline]
    pub fn from_vec(items: Vec<T>) -> Self {
        Self {
            items: Rc::new(RefCell::new(items)),
            emitter: EventEmitter::new(),
        }
    }

    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Self::from_vec(Vec::with_capacity(capacity))
    }

    #[inline]
    pub fn subscribe<E: Element>(&self, id: &TypedId<E>, queue: &EventQueue)
        where
            E::Widget: EventHandler<ListOp<T>>,
            T: 'static
    {
        self.emitter.subscribe(id);
        self.init_subscriber(queue);
    }

    #[inline]
    pub fn create_binding<E: Element>(&self) -> Binding<T>
        where E::Widget: EventHandler<ListOp<T>>
    {
        Binding {
            target: self.clone(),
            inner: self.emitter.create_binding::<E>()
        }
    }

    #[inline]
    pub fn as_slice(&self) -> ListSlice<'_, T> {
        ListSlice(self.items.borrow_mut())
    }

    #[inline]
    #[must_use = "You should give this to ctx.event_queue.schedule()."]
    pub fn push(&self, item: T) -> Event<ListOp<T>>
        where T: 'static
    {
        let items = self.items.clone();

        self.emitter.emit(ListOp::Push(item)).and_then(move |_, event| {
            let ListOp::Push(item) = event else {
                unreachable!();
            };

            items.borrow_mut().push(item);
        })
    }

    #[inline]
    fn init_subscriber(&self, queue: &EventQueue)
        where T: 'static
    {
        let subscriber = self.emitter.last_added().unwrap();
        let op = ListOp::Init(ListRef(self.items.clone()));

        queue.schedule(subscriber.emit(op));
    }
}

impl<T: 'static> Binding<T> {
    #[inline]
    pub fn bind(self, id: impl Into<Id>, queue: &EventQueue) {
        self.inner.bind(id);
        self.target.init_subscriber(queue);
    }
}

impl<T> ListRef<T> {
    #[inline]
    pub fn as_slice(&self) -> ListSlice<'_, T> {
        ListSlice(self.0.borrow_mut())
    }
}

impl<'a, T> Deref for ListSlice<'a, T> {
    type Target = [T];

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.0.as_slice()
    }
}

impl<'a, T> DerefMut for ListSlice<'a, T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.as_mut_slice()
    }
}

impl<T> FromIterator<T> for ReactiveList<T> {
    #[inline]
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let items = Vec::from_iter(iter);

        Self::from_vec(items)
    }
}

impl<T> Clone for ReactiveList<T> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            items: Rc::clone(&self.items),
            emitter: self.emitter.clone()
        }
    }
}

impl<T> From<Vec<T>> for ReactiveList<T> {
    #[inline]
    fn from(items: Vec<T>) -> Self {
        Self::from_vec(items)
    }
}

impl<T: fmt::Debug> fmt::Debug for ReactiveList<T> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.items.borrow().as_slice(), f)
    }
}
