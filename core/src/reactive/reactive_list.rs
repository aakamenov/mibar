use std::{
    cell::{RefCell, RefMut, Ref},
    ops::{Deref, DerefMut},
    hash::{Hash, Hasher},
    rc::Rc,
    fmt,
    mem
};

use ahash::AHasher;
use smallvec::{SmallVec, smallvec};
use nohash::{IntSet, IsEnabled};

use crate::{widget::Element, TypedId, Context, Id, StateHandle};
use super::event_emitter::{EventEmitter, EventHandler};

pub trait ReactiveListHandler<T: HasUniqueKey>: EventHandler<ListOp<T>>
    where Self::State: ReactiveListHandlerState
{ }

pub trait ReactiveListHandlerState {
    type Item: HasId;

    fn child_ids(&mut self) -> &mut SmallVec<[Self::Item; 8]>;
}

pub trait HasId: Default {
    fn id(&self) -> Id;
    fn set_id(&mut self, id: Id);
}

pub trait HasUniqueKey {
    type Key: Hash;

    fn key(&self) -> &Self::Key;
}

pub struct ReactiveList<T: HasUniqueKey> {
    items: Rc<RefCell<State<T>>>,
    emitter: EventEmitter<ListOp<T>>
}

pub enum ListOp<T: HasUniqueKey> {
    Init(ListRef<T>),
    Changes(ListDiff<T>)
}

pub struct NewItems<'a, T: HasUniqueKey> {
    pub list: ListRef<T>,
    pub new_indexes: &'a SmallVec<[usize; 8]>
}

pub struct ListSlice<'a, T>(Ref<'a, State<T>>);
pub struct ListSliceMut<'a, T>(RefMut<'a, State<T>>);

pub struct ListRef<T>(Rc<RefCell<State<T>>>);

pub struct ListDiff<T: HasUniqueKey> {
    list: ListRef<T>,
    diff: Diff
}

#[derive(Debug, Default)]
struct Diff {
    clear: bool,
    added: SmallVec<[usize; 8]>,
    removed: SmallVec<[usize; 8]>,
    moved: SmallVec<[DiffMove; 8]>
}
    
#[derive(Clone, Copy, Debug)]
struct DiffMove {
    from: usize,
    to: usize
}

#[derive(Clone, Copy, Debug)]
struct DiffEntry {
    index: usize,
    key: u64
}

struct State<T> {
    items: Vec<T>,
    items_hashed: IntSet<DiffEntry>
}

impl<T: HasUniqueKey> ReactiveList<T> {
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
        let op = ListOp::Init(ListRef(Rc::clone(&self.items)));

        subscriber.call(ctx, &op);
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.as_slice().len()
    }

    #[inline]
    pub fn as_slice(&self) -> ListSlice<'_, T> {
        ListSlice(self.items.borrow())
    }

    #[inline]
    pub fn as_mut_slice(&self) -> ListSliceMut<'_, T> {
        ListSliceMut(self.items.borrow_mut())
    }

    pub fn push(&self, ctx: &mut Context, item: T) {
        let mut hasher = AHasher::default();
        item.key().hash(&mut hasher);

        let mut state = self.items.borrow_mut();
        state.items.push(item);

        let index = state.items.len() - 1;
        state.items_hashed.insert(DiffEntry {
            index,
            key: hasher.finish()
        });

        drop(state);

        let diff = Diff {
            added: smallvec![index],
            ..Default::default()
        };

        let event = ListOp::Changes(ListDiff {
            list: ListRef(Rc::clone(&self.items)),
            diff
        });

        self.emitter.emit(ctx, &event);
    }

    pub fn mutate(&self, ctx: &mut Context, f: impl FnOnce(&mut Vec<T>)) {
        let mut state = self.items.borrow_mut();
        f(&mut state.items);

        let diff = state.compute_diff();
        let event = ListOp::Changes(ListDiff {
            list: ListRef(Rc::clone(&self.items)),
            diff
        });

        drop(state);

        self.emitter.emit(ctx, &event);
    }
}

impl<T: HasUniqueKey + Clone> ReactiveList<T> {
    pub fn extend_from_slice(&mut self, ctx: &mut Context, slice: &[T]) {
        let mut state = self.items.borrow_mut();
        let start = state.items.len();

        for (index, item) in slice.iter().enumerate() {
            let mut hasher = AHasher::default();
            item.key().hash(&mut hasher);

            state.items_hashed.insert(DiffEntry {
                index: start + index,
                key: hasher.finish()
            });
        }

        state.items.extend_from_slice(slice);
        let new_len = state.items.len();

        drop(state);

        let diff = Diff {
            added: (start..new_len).collect(),
            ..Default::default()
        };

        let event = ListOp::Changes(ListDiff {
            list: ListRef(Rc::clone(&self.items)),
            diff
        });

        self.emitter.emit(ctx, &event);
    }
}

impl<T: HasUniqueKey> State<T> {
    fn new(items: Vec<T>) -> Self {
        let mut state = Self {
            items,
            items_hashed: IntSet::default()
        };

        state.compute_diff();

        state
    }

    fn compute_diff(&mut self) -> Diff {
        if self.items.is_empty() {
            self.items_hashed.clear();

            return Diff {
                clear: true,
                ..Default::default()
            };
        }

        let new = self.items.iter()
            .enumerate()
            .map(|(index, x)| {
                let mut hasher = AHasher::default();
                x.key().hash(&mut hasher);

                DiffEntry {
                    index,
                    key: hasher.finish()
                }
            })
            .collect::<IntSet<DiffEntry>>();

        let removed = self.items_hashed.difference(&new)
            .map(|x| x.index)
            .collect::<SmallVec<[usize; 8]>>();

        let mut added = SmallVec::new();
        let mut moved = SmallVec::new();

        for item in &new {
            if let Some(old) = self.items_hashed.get(item) {
                if old.index != item.index {
                    moved.push(DiffMove {
                        from: old.index,
                        to: item.index
                    });
                }
            } else {
                added.push(item.index);
            }
        }

        self.items_hashed = new;

        Diff {
            clear: false,
            added,
            removed,
            moved
        }
    }
}

impl<T: HasUniqueKey> ListDiff<T> {
    #[must_use = "Use the returned value to generate widgets for any new items."]
    pub fn apply<S: ReactiveListHandlerState + 'static>(
        &self,
        ctx: &mut Context,
        handle: StateHandle<S>
    ) -> NewItems<T> {
        #[derive(Debug)]
        struct MoveOp {
            id: Id,
            to: usize
        }

        let Self { list, diff } = self;

        let items = ctx.tree[handle].child_ids();

        if diff.clear {
            ctx.event_queue.destroy_many(
                items.drain(..).map(|x| x.id())
            );

            return NewItems {
                list: self.list.clone(),
                new_indexes: &diff.added
            };
        }

        // Order is important here:
        // 1. Remove
        // 2. Move
        // 3. Add (performed by the user using the returned NewItems value)

        ctx.event_queue.destroy_many(
            diff.removed.iter().map(|i| items[*i].id())
        );

        let new_len = list.as_slice().len();
        if new_len > items.len() {
            items.resize_with(new_len, Default::default);
        }

        // We can't just directly perform moves on the original items Vec
        // as we might incorrectly overwrite some entries. Consider the scenario
        // where we move from list [A, B] to [B, A]. We'd end up with two move ops
        // that will ultimately cancel eachother when applied.
        let mut move_ops = SmallVec::<[MoveOp; 16]>::new();

        for op in &diff.moved {
            move_ops.push(MoveOp {
                id: mem::take(&mut items[op.from]).id(),
                to: op.to
            });
        }

        for op in move_ops {
            items[op.to].set_id(op.id);
        }

        if new_len < items.len() {
            // At this point we've destroyed all of the removed widgets so we can safely truncate. 
            items.truncate(new_len);
        }

        NewItems {
            list: self.list.clone(),
            new_indexes: &diff.added
        }
    }
}

impl HasId for Id {
    #[inline]
    fn id(&self) -> Id {
        *self
    }

    #[inline]
    fn set_id(&mut self, id: Id) {
        *self = id;
    }
}

impl<T, Item: HasUniqueKey> ReactiveListHandler<Item> for T
    where
        T: EventHandler<ListOp<Item>>,
        T::State: ReactiveListHandlerState { }

impl<T> ListRef<T> {
    #[inline]
    pub fn as_slice(&self) -> ListSlice<'_, T> {
        ListSlice(self.0.borrow())
    }
}

impl<T: HasUniqueKey> Clone for ListRef<T> {
    #[inline]
    fn clone(&self) -> Self {
        Self(Rc::clone(&self.0))
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

impl<T: HasUniqueKey> FromIterator<T> for ReactiveList<T> {
    #[inline]
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let items = Vec::from_iter(iter);

        Self::from_vec(items)
    }
}

impl<T: HasUniqueKey> Clone for ReactiveList<T> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            items: Rc::clone(&self.items),
            emitter: self.emitter.clone()
        }
    }
}

impl<T: HasUniqueKey> From<Vec<T>> for ReactiveList<T> {
    #[inline]
    fn from(items: Vec<T>) -> Self {
        Self::from_vec(items)
    }
}

impl<T: HasUniqueKey + fmt::Debug> fmt::Debug for ReactiveList<T> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.items.borrow().items.as_slice(), f)
    }
}

impl PartialEq for DiffEntry {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
    }
}

impl Eq for DiffEntry { }

impl Hash for DiffEntry {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u64(self.key);
    }
}

// We manually implement Hash above which writes only the u64 key value.
impl IsEnabled for DiffEntry { }

macro_rules! impl_unique_key {
    ($t:ty) => {
        impl HasUniqueKey for $t {
            type Key = Self;

            #[inline]
            fn key(&self) -> &Self::Key {
                self
            }
        }       
    };
}

impl_unique_key!(u8);
impl_unique_key!(u16);
impl_unique_key!(u32);
impl_unique_key!(u64);
impl_unique_key!(u128);
impl_unique_key!(i8);
impl_unique_key!(i16);
impl_unique_key!(i32);
impl_unique_key!(i64);
impl_unique_key!(i128);
impl_unique_key!(&str);
impl_unique_key!(String);
