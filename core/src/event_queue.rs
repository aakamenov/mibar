use std::{cell::RefCell, ptr};

use bumpalo::Bump;

use crate::{Context, Id};

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

/// This trait wraps the FnOnce actions we allocate with bumpalo because it
/// returns mutable references only and FnOnce requires ownership. The invoke()
/// method on this trait creates a copy of the function on the stack and calls
/// the closure. The method is unsafe because we need to ensure that the same
/// closure won't be called again.
pub(crate) trait BumpAllocatedAction {
    unsafe fn invoke(&self, ctx: &mut Context);
}

impl<'a> EventQueue<'a> {
    #[inline]
    pub fn schedule(&self, source: impl EventSource) {
        source.emit(self);
    }

    #[inline]
    pub fn action<F>(&self, f: F)
        where F: FnOnce(&mut Context) + 'a
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

impl<F: FnOnce(&mut Context)> BumpAllocatedAction for F {
    #[inline]
    unsafe fn invoke(&self, ctx: &mut Context) {
        unsafe { ptr::read(self)(ctx) }
    }
}
