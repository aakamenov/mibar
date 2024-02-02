use std::{cell::RefCell, ptr};

use bumpalo::Bump;

use crate::{UpdateCtx, Id};

pub trait EventSource {
    fn emit(self, queue: &EventQueue);
}

pub struct EventQueue<'a> {
    pub(crate) alloc: &'a Bump,
    pub(crate) buffer: &'a RefCell<Vec<EventType<'a>>>
}

pub(crate) enum EventType<'a> {
    Action(&'a dyn BumpAllocatedAction),
    Destroy(Id)
}

pub(crate) unsafe trait BumpAllocatedAction {
    fn invoke(&self, ctx: &mut UpdateCtx);
}

impl<'a> EventQueue<'a> {
    #[inline]
    pub fn schedule(&self, source: impl EventSource) {
        source.emit(self);
    }

    #[inline]
    pub fn action<F>(&self, f: F)
        where F: FnOnce(&mut UpdateCtx) + 'a
    {
        let action = self.alloc.alloc(f);
        self.buffer.borrow_mut().push(EventType::Action(action));
    }

    #[inline]
    pub fn destroy(&self, id: Id) {
        self.buffer.borrow_mut().push(EventType::Destroy(id));
    }
}

impl<'a> Clone for EventQueue<'a> {
    fn clone(&self) -> Self {
        EventQueue {
            alloc: self.alloc,
            buffer: self.buffer
        }
    }
}

impl<'a> Copy for EventQueue<'a> { }

unsafe impl<F: FnOnce(&mut UpdateCtx)> BumpAllocatedAction for F {
    #[inline]
    fn invoke(&self, ctx: &mut UpdateCtx) {
        unsafe { ptr::read(self)(ctx) }
    }
}
