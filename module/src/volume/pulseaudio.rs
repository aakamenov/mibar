use std::{
    fmt,
    thread,
    rc::Rc,
    cell::RefCell,
    sync::{RwLock, atomic::{AtomicU8, Ordering}, mpsc},
    ops::Deref
};

use tokio::sync::watch;
use pulse::{
    def::Retval,
    volume::Volume,
    context::{
        self, Context,
        introspect::SinkInfo,
        subscribe::{self, InterestMaskSet, Facility}
    },
    mainloop::{
        api::Mainloop,
        threaded::Mainloop as ThreadedLoop
    },
    proplist::{properties, Proplist},
    callbacks::ListResult
};

type ContextRc = Rc<RefCell<Context>>;

const DEFAULT_SINK: &str = "@DEFAULT_SINK@";

const UNINITIALIZED: u8 = 0;
const INITIALIZING: u8 = 1;
const INITIALIZED: u8 = 2;

static CLIENT_STATE: AtomicU8 = AtomicU8::new(UNINITIALIZED);

static REQUEST_DISPATCHER: RwLock<Option<mpsc::Sender<Request>>> =
    RwLock::new(None);

static SENDER: RwLock<Option<watch::Sender<State>>> =
    RwLock::new(None);

static RECEIVER: RwLock<Option<watch::Receiver<State>>> =
    RwLock::new(None);

static DEFAULT_SINK_IDX: RwLock<Option<u32>> = RwLock::new(None);

// ------------------------------------------------------------------
//                         IMPORTANT!!!
//
// The values below must only be utilized inside the callbacks
// (which are all executed within a single thread).

/// We use this in the callback when querying the default sink
/// since PulseAudio calls it multiple times with different
/// enumeration states. If the enumerations state is End or Error
/// and this is set to false that means that there is no default sink
/// and we set DEFAULT_SINK_IDX to None.
static mut HAS_DEFAULT_SINK: bool = false;
// -------------------------------------------------------------------

pub enum Request {
    ToggleMute,
    Terminate
}

#[derive(Clone, Copy, PartialEq, Default, Debug)]
pub struct State {
    pub volume: u8,
    pub is_muted: bool
}

#[inline]
pub fn subscribe() -> Option<watch::Receiver<State>> {
    let lock = RECEIVER.read().unwrap();

    lock.deref().clone()
}

#[inline]
pub fn subscriber_count() -> usize {
    let lock = SENDER.read().unwrap();
    if let Some(tx) = lock.deref() {
        tx.receiver_count() - 1
    } else {
        0
    }
}

#[inline]
pub fn dispatch(request: Request) -> bool {
    match REQUEST_DISPATCHER.read().unwrap().deref() {
        Some(dispatcher) => {
            match dispatcher.send(request) {
                Ok(_) => true,
                Err(_) => false
            }
        }
        None => false
    }
}

pub fn init() {
    if let Err(_) = CLIENT_STATE.compare_exchange(
        UNINITIALIZED,
        INITIALIZING,
        Ordering::AcqRel,
        Ordering::Acquire
    ) {
        return;
    }
    
    // Create the channels now in order to make them readily available
    // and not have to wait for the connection with the server to be established.
    let (tx, rx) = watch::channel(State::default());
    *SENDER.try_write().unwrap() = Some(tx);
    *RECEIVER.try_write().unwrap() = Some(rx);

    start_dispatcher_thread();
}

fn start_dispatcher_thread() {
    let (tx, rx) = mpsc::channel();

    // This sender is later synchronously released in state_callback(),
    // causing the  thread that we spawn below to exit.
    *REQUEST_DISPATCHER.try_write().unwrap() = Some(tx);

    thread::spawn(move || {
        #[inline]
        fn error_and_cleanup(args: fmt::Arguments) {
            use std::io::{Write, stderr};
            stderr().write_fmt(args).expect("Write stderr");

            cleanup();
            CLIENT_STATE.store(UNINITIALIZED, Ordering::Release);
        }

        let mut proplist = Proplist::new().unwrap();
        if let Err(()) = proplist.set_str(properties::APPLICATION_NAME, "mibar") {
            eprintln!("Error setting the PulseAudio application.name property.");
        }

        let Some(mut main_loop) = ThreadedLoop::new() else {
            error_and_cleanup(
                format_args!("Failed to create a PulseAudio event loop.")
            );

            return;
        };

        let Some(ctx) = Context::new_with_proplist(
            &main_loop,
            "MibarAppContext",
            &proplist
        ) else {
            error_and_cleanup(
                format_args!("Failed to create PulseAudio context.")
            );

            return;
        };

        let ctx = Rc::new(RefCell::new(ctx));
        state_callback(&ctx);
        subscribe_callback(&ctx);

        if let Err(err) = ctx.borrow_mut().connect(
            None,
            context::FlagSet::NOFLAGS,
            None
        ) {
            error_and_cleanup(
                format_args!(
                    "Error connecting to the PulseAudio server: {}",
                    err.to_string().unwrap_or_default()
                )
            );

            return;
        }

        main_loop.lock();
        if let Err(err) = main_loop.start() {
            error_and_cleanup(
                format_args!(
                    "Error starting the PulseAudio loop: {}",
                    err.to_string().unwrap_or_default()
                )
            );

            return;
        };
        main_loop.unlock();

        let mut introspector = ctx.borrow().introspect();

        // When the client is quitting, REQUEST_DISPATCHER gets dropped in
        // state_callback(), causing this loop to break out.
        while let Ok(request) = rx.recv() {
            let lock = RECEIVER.read().unwrap();
            let Some(state) = lock.as_ref()
                .and_then(|x| Some(x.borrow().clone())) else
            {
                continue;
            };

            drop(lock);

            let Some(index) = *DEFAULT_SINK_IDX.read().unwrap() else {
                continue;
            };

            match request {
                Request::ToggleMute => {
                    introspector.set_sink_mute_by_index(
                        index,
                        !state.is_muted,
                        None
                    );
                }
                Request::Terminate => {
                    if let Some(client_index) = ctx.borrow().get_index() {
                        introspector.kill_client(
                            client_index,
                            |_| ()
                        );
                    }
                }
            };
        }

        main_loop.quit(Retval(0));

        cleanup();
        CLIENT_STATE.store(UNINITIALIZED, Ordering::Release);
    });
}

fn state_callback(ctx_ref: &ContextRc) {
    let ctx = Rc::clone(ctx_ref);
    ctx_ref.borrow_mut().set_state_callback(Some(Box::new(move || {
        let state = unsafe { (*ctx.as_ptr()).get_state() };
        let quit = match state {
            context::State::Ready => {
                CLIENT_STATE.store(INITIALIZED, Ordering::Release);

                let mut ctx = ctx.borrow_mut();
                ctx.introspect()
                    .get_sink_info_by_name(
                        DEFAULT_SINK,
                        default_sink_info_callback
                    );

                ctx.subscribe(
                    InterestMaskSet::SERVER | InterestMaskSet::SINK,
                    |_| ()
                );

                false
            }
            context::State::Failed => true,
            context::State::Terminated => true,
            _ => false
        };

        if quit {
            // Now that we drop REQUEST_DISPATCHER, the dispatcher thread
            // should safely exit and reset the global state.
            *REQUEST_DISPATCHER.write().unwrap() = None;
        }
    })));
}

fn subscribe_callback(ctx_ref: &ContextRc) {
    let ctx = Rc::clone(ctx_ref);
    ctx_ref.borrow_mut().set_subscribe_callback(
        Some(Box::new(move |facility, op, index| {
            let (Some(facility), Some(op)) = (facility, op) else {
                return;
            };

            let is_current = DEFAULT_SINK_IDX.read()
                .unwrap()
                .map_or(false, |i| i == index);

            match facility {
                Facility::Server => {
                     ctx.borrow_mut()
                        .introspect()
                        .get_sink_info_by_name(
                            DEFAULT_SINK,
                            default_sink_info_callback
                        );
                }
                Facility::Sink if op == subscribe::Operation::Changed && is_current => {
                    ctx.borrow_mut()
                        .introspect()
                        .get_sink_info_by_index(index, sink_info_callback);
                }
                Facility::Sink if op == subscribe::Operation::Removed && is_current => {
                    ctx.borrow_mut()
                        .introspect()
                        .get_sink_info_by_name(
                            DEFAULT_SINK,
                            default_sink_info_callback
                        );
                }
                _ => { }
            }
        }))
    );
}

fn sink_info_callback(result: ListResult<&SinkInfo>) {
    if let ListResult::Item(sink) = result {
        update_volume(sink);
    }
}

fn default_sink_info_callback(result: ListResult<&SinkInfo>) {
    let mut index = DEFAULT_SINK_IDX.write().unwrap();
    match result {
        ListResult::Item(sink) => unsafe {
            HAS_DEFAULT_SINK = true;
            *index = Some(sink.index);

            update_volume(sink);
        }
        ListResult::End | ListResult::Error => unsafe {
            if !HAS_DEFAULT_SINK {
                // No default sink.
                *index = None;
            }

            HAS_DEFAULT_SINK = false;
        }
    }
}

fn update_volume(sink: &SinkInfo) {
    let percent = sink.volume.max().0 as f32 * 100f32 /
        Volume::NORMAL.0 as f32 + 0.5;

    let lock = SENDER.read().unwrap();
    if let Some(sender) = lock.as_ref().and_then(|x| Some(x)) {
        sender.send_modify(|val| {
            val.volume = percent as u8;
            val.is_muted = sink.mute;
        });
    }
}

fn cleanup() {
    *REQUEST_DISPATCHER.write().unwrap() = None;
    *SENDER.write().unwrap() = None;
    *RECEIVER.write().unwrap() = None;
    *DEFAULT_SINK_IDX.write().unwrap() = None;
}
