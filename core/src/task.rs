use std::{future::Future, marker::PhantomData};

use tokio::task::JoinHandle;

use crate::{UiCtx, Id, ValueSender};

pub struct Task;

pub trait AsyncTask {
    fn spawn(self, ctx: &UiCtx) -> JoinHandle<()>;
}

pub struct Void<
    F: Future<Output = ()> + Send + 'static
> {
    task: F
}

pub struct Result<
    T,
    F: Future<Output = T> + Send + 'static
> {
    id: Id,
    task: F
}

pub struct MultiResult<
    T: Send + 'static,
    C: FnOnce(ValueSender<T>) -> F,
    F: Future<Output = ()> + Send + 'static
> {
    id: Id,
    create: C,
    data: PhantomData<T>
}

impl Task {
    /// A fire and forget type task that does not produce any result.
    /// This will **not** call [`Widget::task_result`] when complete.
    #[inline]
    #[must_use = "You should give this to ctx.ui.spawn()."]
    pub fn void<F: Future<Output = ()> + Send + 'static>(task: F) -> Void<F> {
        Void { task }
    }

    /// A task that produces a single value and when complete calls
    /// [`Widget::task_result`] on the widget that initiated this method with the
    /// value produced by the async computation. You MUST implement
    /// [`Widget::task_result`] if you are using this method in your widget. If you
    /// don't, the default implementation is a panic which will remind you of that.
    #[inline]
    #[must_use = "You should give this to ctx.ui.spawn()."]
    pub fn new<
        T: Send + 'static,
        F: Future<Output = T> + Send + 'static
    >(id: impl Into<Id>, task: F) -> Result<T, F> {
        Result {
            id: id.into(),
            task
        }
    }

    /// A task that can produce multiple values. For each value produced
    /// [`Widget::task_result`] is called on the widget that initiated this method
    /// with the value sent by the `ValueSender`. You MUST implement
    /// [`Widget::task_result`] if you are using this method in your widget. If you
    /// don't, the default implementation is a panic which will remind you of that.
    #[inline]
    #[must_use = "You should give this to ctx.ui.spawn()."]
    pub fn with_sender<
        T: Send + 'static,
        C: FnOnce(ValueSender<T>) -> F,
        F: Future<Output = ()> + Send + 'static
    >(id: impl Into<Id>, create: C) -> MultiResult<T, C, F> {
        MultiResult {
            id: id.into(),
            create,
            data: PhantomData
        }
    }
}

impl<
    F: Future<Output = ()> + Send + 'static
> AsyncTask for  Void<F> {
    #[inline]
    fn spawn(self, ctx: &UiCtx) -> JoinHandle<()> {
        ctx.runtime_handle().spawn(self.task)
    }
}

impl<
    T: Send + 'static,
    F: Future<Output = T> + Send + 'static
> AsyncTask for Result<T, F> {
    #[inline]
    fn spawn(self, ctx: &UiCtx) -> JoinHandle<()> {
        let sender = ctx.value_sender(self.id);
        let window_id = ctx.window_id();

        ctx.runtime_handle().spawn(async move {
            let result = self.task.await;

            if !sender.send(result) {
                eprintln!("Failed to send task result to window {:?} - it has already closed.", window_id);
            }
        })
    }
}

impl<
    T: Send + 'static,
    C: FnOnce(ValueSender<T>) -> F,
    F: Future<Output = ()> + Send + 'static
> AsyncTask for MultiResult<T, C, F> {
    #[inline]
    fn spawn(self, ctx: &UiCtx) -> JoinHandle<()> {
        let sender = ctx.value_sender(self.id);
        ctx.runtime_handle().spawn((self.create)(sender))
    }
}
