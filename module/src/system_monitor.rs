use std::{
    pin::Pin,
    future::Future,
    sync::Mutex
};

use tokio::{
    sync::watch::{self, Sender, Ref, error::RecvError, channel},
    task::JoinHandle,
    runtime
};

pub trait Listener {
    type Value: Send + Sync;

    fn initial_value() -> Self::Value;
    fn run(tx: Sender<Self::Value>) -> Pin<Box<dyn Future<Output = ()> + Send>>;
}

// This type should only be used as a `static` variable
// due to how we are handling unsubscriptions!!!
pub struct SystemMonitor<T: Listener> {
    state: Mutex<Option<State<T>>>
}

pub struct Receiver<T: Listener> {
    rx: watch::Receiver<T::Value>,
    handle: UnsubscribeHandle<T> 
}

struct State<T: Listener> {
    subscribers: u8,
    rx: watch::Receiver<T::Value>,
    handle: JoinHandle<()>
}

struct UnsubscribeHandle<T: Listener>(*const SystemMonitor<T>);

impl<T: Listener> SystemMonitor<T> {
    pub const fn new() -> Self {
        Self {
            state: Mutex::new(None)
        }
    }

    pub fn subscribe(&self, handle: &runtime::Handle) -> Receiver<T> {
        let mut state = self.state.lock().unwrap();

        match state.as_mut() {
            Some(state) => {
                state.subscribers += 1;

                Receiver {
                    rx: state.rx.clone(),
                    handle: UnsubscribeHandle(self)
                }
            }
            None => {
                let (tx, rx) = channel(T::initial_value());
                let handle = handle.spawn(T::run(tx));

                *state = Some(State {
                    subscribers: 1,
                    rx: rx.clone(),
                    handle
                });

                Receiver {
                    rx,
                    handle: UnsubscribeHandle(self)
                }
            }
        }
    }

    fn unsubscribe(&self) {
        let mut lock = self.state.lock().unwrap();

        if let Some(state) = lock.as_mut() {
            state.subscribers -= 1;

            if state.subscribers == 0 {
                state.handle.abort();
            }

            *lock = None;
        }
    }
}

impl<T: Listener> Receiver<T> {
    #[inline]
    pub async fn recv(&mut self) -> Result<Ref<T::Value>, RecvError> {
        self.rx.changed().await?;

        Ok(self.rx.borrow_and_update())
    }
}

impl<T: Listener> Drop for Receiver<T> {
    fn drop(&mut self) {
        let monitor = unsafe { &*(self.handle.0) };
        monitor.unsubscribe();
    }
}

unsafe impl<T: Listener> Sync for UnsubscribeHandle<T> { }
unsafe impl<T: Listener> Send for UnsubscribeHandle<T> { }
