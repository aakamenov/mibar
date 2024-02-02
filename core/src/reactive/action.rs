use std::rc::Rc;

use crate::{UpdateCtx, EventQueue, EventSource};

#[derive(Clone)]
pub struct Action(Rc<dyn Fn(&mut UpdateCtx)>);

impl EventSource for Action {
    #[inline]
    fn emit(self, queue: &EventQueue) {
        queue.action(move |ctx| self.0(ctx));
    }
}

impl EventSource for &Action {
    #[inline]
    fn emit(self, queue: &EventQueue) {
        let action = self.0.clone();
        queue.action(move |ctx| action(ctx));
    }
}

impl Action {
    #[inline]
    pub fn new(action: impl Fn(&mut UpdateCtx) + 'static) -> Self {
        Self(Rc::new(action))
    }
}

impl<F: Fn(&mut UpdateCtx) + 'static> From<F> for Action {
    #[inline]
    fn from(action: F) -> Self {
        Self(Rc::new(action))
    }
}
