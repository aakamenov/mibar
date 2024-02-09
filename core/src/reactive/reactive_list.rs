use std::{
    cell::{RefCell, RefMut, Ref},
    ops::{Deref, DerefMut},
    hash::{Hash, Hasher},
    rc::Rc,
    fmt
};

use ahash::AHasher;
use nohash::{IntSet, IsEnabled};

use crate::{widget::Element, TypedId, Context};
use super::event_emitter::{EventEmitter, EventHandler};

pub trait ReactiveListHandler<T: UniqueKey>: EventHandler<ListOp<T>>
    where Self::State: ReactiveListHandlerState<T>
{ }

pub trait ReactiveListHandlerState<T: UniqueKey> {
    fn add(&mut self, ctx: &mut Context, item: &T);
}

pub trait UniqueKey: Hash + Eq { }

pub struct ReactiveList<T: UniqueKey> {
    items: Rc<RefCell<State<T>>>,
    emitter: EventEmitter<ListOp<T>>
}

pub enum ListOp<T: UniqueKey> {
    Init(ListRef<T>),
    Changes(ListDiff<T>)
}

pub struct ListDiff<T: UniqueKey> {
    state: Rc<RefCell<State<T>>>,
    diff: Vec<DiffEntry>
}

pub struct ListSlice<'a, T>(Ref<'a, State<T>>);
pub struct ListSliceMut<'a, T>(RefMut<'a, State<T>>);

pub struct ListRef<T>(Rc<RefCell<State<T>>>);

struct State<T> {
    items: Vec<T>,
    items_hashed: IntSet<DiffEntry>
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct DiffEntry {
    index: usize,
    key: u64
}

impl<T: UniqueKey> ReactiveList<T> {
    #[inline]
    pub fn new() -> Self {
        Self::from_vec(Vec::new())
    }

    #[inline]
    pub fn from_vec(items: Vec<T>) -> Self {
        Self {
            items: Rc::new(RefCell::new(State::new(items))),
            emitter: EventEmitter::new(),
        }
    }

    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Self::from_vec(Vec::with_capacity(capacity))
    }

    #[inline]
    pub fn subscribe<E: Element>(&self, ctx: &mut Context, id: TypedId<E>)
        where
            E::Widget: EventHandler<ListOp<T>>,
            T: 'static
    {
        self.emitter.subscribe(id);

        let subscriber = self.emitter.last_added().unwrap();
        let op = ListOp::Init(ListRef(self.items.clone()));

        subscriber.call(ctx, &op);
    }

    #[inline]
    pub fn as_slice(&self) -> ListSlice<'_, T> {
        ListSlice(self.items.borrow())
    }

    #[inline]
    pub fn as_mut_slice(&self) -> ListSliceMut<'_, T> {
        ListSliceMut(self.items.borrow_mut())
    }

    pub fn mutate(&self, ctx: &mut Context, f: impl FnOnce(&mut Vec<T>)) {
        let mut state = self.items.borrow_mut();
        f(&mut state.items);

        let diff = state.compute_diff();
        let event = ListOp::Changes(ListDiff {
            state: self.items.clone(),
            diff
        });

        self.emitter.emit(ctx, &event);
    }
}

impl<T: UniqueKey> State<T> {
    fn new(items: Vec<T>) -> Self {
        let mut state = Self {
            items,
            items_hashed: IntSet::default()
        };

        state.compute_diff();

        state
    }

    fn compute_diff(&mut self) -> Vec<DiffEntry> {
        let new = self.items.iter()
            .enumerate()
            .map(|(index, x)| {
                let mut hasher = AHasher::default();
                x.hash(&mut hasher);

                DiffEntry {
                    index,
                    key: hasher.finish()
                }
            })
            .collect::<IntSet<DiffEntry>>();

        let diff = new.difference(&self.items_hashed).copied().collect();
        self.items_hashed = new;

        diff
    }
}

impl<T, Item: UniqueKey> ReactiveListHandler<Item> for T
    where
        T: EventHandler<ListOp<Item>>,
        T::State: ReactiveListHandlerState<Item> { }

impl<T> ListRef<T> {
    #[inline]
    pub fn as_slice(&self) -> ListSlice<'_, T> {
        ListSlice(self.0.borrow())
    }
}

impl<'a, T> Deref for ListSlice<'a, T> {
    type Target = [T];

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.0.items.as_slice()
    }
}

impl<'a, T> Deref for ListSliceMut<'a, T> {
    type Target = [T];

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.0.items.as_slice()
    }
}

impl<'a, T> DerefMut for ListSliceMut<'a, T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.items.as_mut_slice()
    }
}

impl<T: UniqueKey> FromIterator<T> for ReactiveList<T> {
    #[inline]
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let items = Vec::from_iter(iter);

        Self::from_vec(items)
    }
}

impl<T: UniqueKey> Clone for ReactiveList<T> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            items: Rc::clone(&self.items),
            emitter: self.emitter.clone()
        }
    }
}

impl<T: UniqueKey> From<Vec<T>> for ReactiveList<T> {
    #[inline]
    fn from(items: Vec<T>) -> Self {
        Self::from_vec(items)
    }
}

impl<T: UniqueKey + fmt::Debug> fmt::Debug for ReactiveList<T> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.items.borrow().items.as_slice(), f)
    }
}

impl<T: Hash + Eq> UniqueKey for T { }

impl Hash for DiffEntry {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u64(self.key);
    }
}

impl IsEnabled for DiffEntry { }
