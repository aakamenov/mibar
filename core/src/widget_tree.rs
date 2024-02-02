use std::{
    fmt,
    rc::Rc,
    any::Any,
    marker::PhantomData,
    ops::{Index, IndexMut}
};

use slotmap::{SlotMap, SecondaryMap, Key, new_key_type};
use smallvec::SmallVec;

use crate::{widget::AnyWidget, Rect, Id};

pub trait ProvidesContext<C> {
    fn context_handle(&self) -> ContextHandle<C>;
}

pub trait ContextLens<C, T>: ProvidesContext<C> {
    fn lens(context: &mut C) -> &mut T;
}

pub struct StateHandle<T> {
    pub(crate) id: RawWidgetId,
    data: PhantomData<T>
}

pub struct ContextHandle<T> {
    pub(crate) id: RawContextId,
    data: PhantomData<T>
}

pub struct WidgetTree {
    pub(crate) widgets: SlotMap<RawWidgetId, WidgetState>,
    pub(crate) parent_to_children: SecondaryMap<RawWidgetId, SmallVec<[RawWidgetId; 4]>>,
    pub(crate) child_to_parent: SecondaryMap<RawWidgetId, RawWidgetId>,
    pub(crate) contexts: SlotMap<RawContextId, Box<dyn Any + 'static>>,
    pub(crate) dependencies: SecondaryMap<RawWidgetId, SmallVec<[RawContextId; 2]>>
}

new_key_type! {
    pub(crate) struct RawWidgetId;
    pub(crate) struct RawContextId;
}

pub(crate) struct WidgetState {
    pub widget: Rc<dyn AnyWidget>,
    pub state: Box<dyn Any + 'static>,
    pub layout: Rect
}

impl WidgetTree {
    pub(crate) fn new() -> Self {
        Self {
            widgets: SlotMap::with_capacity_and_key(20),
            parent_to_children: SecondaryMap::new(),
            child_to_parent: SecondaryMap::new(),
            contexts: SlotMap::with_key(),
            dependencies: SecondaryMap::new()
        }
    }

    #[inline]
    pub fn state<S: 'static>(&self, handle: StateHandle<S>) -> Option<&S> {
        self.widgets.get(handle.id)
            .map(|x| x.state.downcast_ref().unwrap())
    }

    #[inline]
    pub fn state_mut<S: 'static>(&mut self, handle: StateHandle<S>) -> Option<&mut S> {
        self.widgets.get_mut(handle.id)
            .map(|x| x.state.downcast_mut().unwrap())
    }

    #[inline]
    pub fn state_and_context<S: 'static, C: 'static>(
        &mut self,
        state: StateHandle<S>,
        context: ContextHandle<C>
    ) -> (&mut S, &mut C) {
        (
            self.widgets[state.id].state.downcast_mut().unwrap(),
            self.contexts[context.id].downcast_mut().unwrap(),
        )
    }

    #[inline]
    pub fn state_with_context<S: 'static, C: 'static>(
        &mut self,
        state: StateHandle<S>,
        get: impl FnOnce(&S) -> ContextHandle<C>
    ) -> (&mut S, &mut C) {
        let state = self.widgets[state.id].state.downcast_mut().unwrap();
        let context = self.contexts[get(&state).id].downcast_mut().unwrap();

        (state, context)
    }

    #[inline]
    pub fn context<C: 'static>(&self, handle: ContextHandle<C>) -> Option<&C> {
        self.contexts.get(handle.id)
            .map(|x| x.downcast_ref().unwrap())
    }

    #[inline]
    pub fn context_mut<C: 'static>(&mut self, handle: ContextHandle<C>) -> Option<&mut C> {
        self.contexts.get_mut(handle.id)
            .map(|x| x.downcast_mut().unwrap())
    }

    #[inline]
    pub fn set_context<C: 'static>(&mut self, context: C) -> ContextHandle<C> {
        let id = self.contexts.insert(Box::new(context));

        ContextHandle::new(id)
    }

    #[inline]
    pub fn set_context_dependency<C: 'static>(
        &mut self,
        owner: Id,
        context: C
    ) -> ContextHandle<C> {
        let id = self.contexts.insert(Box::new(context));
        let handle = ContextHandle::new(id);

        let entry = self.dependencies.entry(owner.0)
            .expect("Provided widget ID is not alive.")
            .or_default();

        entry.push(handle.id);

        handle
    }

    #[inline]
    pub fn remove_context<C: 'static>(&mut self, handle: ContextHandle<C>) -> Option<C> {
        self.contexts.remove(handle.id)
            .map(|x| *x.downcast().unwrap())
    }

    pub(crate) fn dealloc(&mut self, id: RawWidgetId) {
        let widget = self.widgets.remove(id).unwrap();

        let parent = self.child_to_parent.remove(id).unwrap();
        if parent != RawWidgetId::null() {
            let children = &mut self.parent_to_children[parent];
            let index = children.iter().position(|x| *x == id).unwrap();

            // Can't use swap_remove() because order is important when doing layout.
            children.remove(index);
        }

        let children = self.parent_to_children
            .remove(id)
            .unwrap_or_default();

        for child in children {
            self.dealloc_impl(child);
        }

        // We call destroy after we've deallocated the children as
        // they could be relying on the parent being alive during their destroy() call.
        widget.widget.destroy(widget.state);

        for id in self.dependencies.remove(id).unwrap_or_default() {
            self.contexts.remove(id);
        }
    }

    fn dealloc_impl(&mut self, id: RawWidgetId) {
        let widget = self.widgets.remove(id).unwrap();

        self.child_to_parent.remove(id).unwrap();

        let children = self.parent_to_children
            .remove(id)
            .unwrap_or_default();

        for child in children {
            self.dealloc_impl(child);
        }

        // We call destroy after we've deallocated the children as
        // they could be relying on the parent being alive during their destroy() call.
        widget.widget.destroy(widget.state);

        for id in self.dependencies.remove(id).unwrap_or_default() {
            self.contexts.remove(id);
        }
    }
}

impl<T: 'static> Index<StateHandle<T>> for WidgetTree {
    type Output = T;

    fn index(&self, handle: StateHandle<T>) -> &Self::Output {
        self.widgets[handle.id].state.downcast_ref().unwrap()
    }
}

impl<T: 'static> IndexMut<StateHandle<T>> for WidgetTree {
    fn index_mut(&mut self, handle: StateHandle<T>) -> &mut Self::Output {
        self.widgets[handle.id].state.downcast_mut().unwrap()
    }
}

impl<T: 'static> Index<ContextHandle<T>> for WidgetTree {
    type Output = T;

    fn index(&self, handle: ContextHandle<T>) -> &Self::Output {
        self.contexts[handle.id].downcast_ref().unwrap()
    }
}

impl<T: 'static> IndexMut<ContextHandle<T>> for WidgetTree {
    fn index_mut(&mut self, handle: ContextHandle<T>) -> &mut Self::Output {
        self.contexts[handle.id].downcast_mut().unwrap()
    }
}

impl WidgetState {
    #[inline]
    pub fn new(widget: Rc<dyn AnyWidget>, state: Box<dyn Any + 'static>) -> Self {
        Self {
            widget,
            state,
            layout: Rect::default()
        }
    }
}

impl<T> StateHandle<T> {
    #[inline]
    pub(crate) fn new(id: RawWidgetId) -> Self {
        Self {
            id,
            data: PhantomData
        }
    }
}

impl<T> ContextHandle<T> {
    #[inline]
    pub(crate) fn new(id: RawContextId) -> Self {
        Self {
            id,
            data: PhantomData
        }
    }
}

impl<T> Clone for StateHandle<T> {
    #[inline]
    fn clone(&self) -> Self {
        Self::new(self.id)
    }
}

impl<T> Copy for StateHandle<T> { }

impl<T> fmt::Debug for StateHandle<T> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.id, f)
    }
}

impl<T> Clone for ContextHandle<T> {
    #[inline]
    fn clone(&self) -> Self {
        Self::new(self.id)
    }
}

impl<T> Copy for ContextHandle<T> { }

impl<T> fmt::Debug for ContextHandle<T> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.id, f)
    }
}

impl<C> ProvidesContext<C> for ContextHandle<C> {
    #[inline]
    fn context_handle(&self) -> ContextHandle<C> {
        *self
    }
}

impl<C> ContextLens<C, C> for ContextHandle<C> {
    #[inline]
    fn lens(context: &mut C) -> &mut C {
        context
    }
}
