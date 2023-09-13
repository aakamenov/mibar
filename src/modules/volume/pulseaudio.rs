use std::{
    fmt,
    thread::{self, JoinHandle},
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
        introspect::{Introspector, SinkInfo},
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
type LoopRc = Rc<RefCell<ThreadedLoop>>;

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
// (which are all executed within a single thread) or inside the
// init() function, using only simple assignments and reads.

/// We use this in the callback when querying the default sink
/// since PulseAudio calls it multiple times with different
/// enumeration states. If the enumerations state is End or Error
/// and this is set to false that means that there is no default sink
/// and we set DEFAULT_SINK_IDX to None.
static mut HAS_DEFAULT_SINK: bool = false;

static mut DISPATCH_THREAD_HANDLE: Option<JoinHandle<()>> = None;
// -------------------------------------------------------------------

pub enum Request {
    ToggleMute
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
    #[inline]
    fn error_and_cleanup(ctx: Option<&mut Context>, args: fmt::Arguments) {
        use std::io::{Write, stderr};
        stderr().write_fmt(args).expect("Write stderr");

        cleanup(ctx);
        CLIENT_STATE.store(UNINITIALIZED, Ordering::Release);
    }

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

    let mut proplist = Proplist::new().unwrap();
    if let Err(()) = proplist.set_str(properties::APPLICATION_NAME, "mibar") {
        eprintln!("Error setting the PulseAudio application.name property.");
    }

    let Some(main_loop) = ThreadedLoop::new() else {
        error_and_cleanup(
            None,
            format_args!("Failed to create a PulseAudio event loop.")
        );

        return;
    };

    let main_loop = Rc::new(RefCell::new(main_loop));

    let Some(ctx) = Context::new_with_proplist(
        main_loop.borrow().deref(),
        "MibarAppContext",
        &proplist
    ) else {
        error_and_cleanup(
            None,
            format_args!("Failed to create PulseAudio context.")
        );

        return;
    };

    let introspector = ctx.introspect();
    let ctx = Rc::new(RefCell::new(ctx));
    state_callback(&ctx, Rc::clone(&main_loop));
    subscribe_callback(&ctx);

    if let Err(err) = ctx.borrow_mut().connect(
        None,
        context::FlagSet::NOFLAGS,
        None
    ) {
        error_and_cleanup(
            Some(&mut ctx.borrow_mut()),
            format_args!(
                "Error connecting to the PulseAudio server: {}",
                err.to_string().unwrap_or_default()
            )
        );

        return;
    }

    main_loop.borrow_mut().lock();
    if let Err(err) = main_loop.borrow_mut().start() {
        error_and_cleanup(
            Some(&mut ctx.borrow_mut()),
            format_args!(
                "Error starting the PulseAudio loop: {}",
                err.to_string().unwrap_or_default()
            )
        );

        return;
    };

    // We start the dispatcher thread here for the same reason as
    // with the globals above - to allow requests to be immediately
    // buffered after this call. However, it is important to do
    // so before the unlock() call below in order to ensure that
    // the state callback is only called AFTER we have assigned
    // to DISPATCH_THREAD_HANDLE.
    unsafe {
        DISPATCH_THREAD_HANDLE = Some(start_dispatcher_thread(
            introspector
        ));
    }

    main_loop.borrow_mut().unlock();
}

fn start_dispatcher_thread(mut introspector: Introspector) -> JoinHandle<()> {
    let (tx, rx) = mpsc::channel();

    // This sender is later synchronously released in state_callback()
    // together with the other globals before marking the client state
    // as uninitialized. Otherwise, it's possible to try initing the client
    // again and we may face the ABA problem.
    *REQUEST_DISPATCHER.try_write().unwrap() = Some(tx);

    // The introspector object internally decrements the context ref count
    // when it's dropped which will happen when the client is terminating in
    // state_callback() as REQUEST_DISPATCHER gets set to None and the while
    // loop below terminates.
    thread::spawn(move || {
        while let Ok(request) = rx.recv() {
            let lock = RECEIVER.read().unwrap();
            let Some(state) = lock.as_ref()
                .and_then(|x| Some(x.borrow().clone())) else
            {
                break;
            };

            drop(lock);

            let Some(index) = *DEFAULT_SINK_IDX.read().unwrap() else {
                continue;
            };

            match request {
                Request::ToggleMute => introspector.set_sink_mute_by_index(
                    index,
                    !state.is_muted,
                    None
                )
            };
        }
    })
}

fn state_callback(ctx_ref: &ContextRc, main_loop: LoopRc) {
    let ctx = Rc::clone(ctx_ref);

    ctx_ref.borrow_mut().set_state_callback(Some(Box::new(move || {
        let state = unsafe { (*ctx.as_ptr()).get_state() };
        let retval = match state {
            context::State::Ready => {
                CLIENT_STATE.store(INITIALIZED, Ordering::Release);

                let mut ctx = ctx.borrow_mut();

                let introspect = ctx.introspect();
                introspect.get_sink_info_by_name(
                    DEFAULT_SINK,
                    default_sink_info_callback
                );

                // Populate state with initial info
                introspect.get_sink_info_by_name(
                    DEFAULT_SINK,
                    sink_info_callback
                );

                ctx.subscribe(
                    InterestMaskSet::SERVER | InterestMaskSet::SINK,
                    |_| ()
                );

                None
            }
            context::State::Failed => Some(Retval(1)),
            context::State::Terminated => Some(Retval(0)),
            _ => None
        };

        if let Some(retval) = retval {
            main_loop.borrow_mut().quit(retval);
            cleanup(Some(&mut ctx.borrow_mut()));

            // Now that we've dropped REQUEST_DISPATCHER in cleanup(),
            // the dispatcher thread should exit and we safely wait for
            // it to do so.
            if let Some(handle) = unsafe { DISPATCH_THREAD_HANDLE.take() } {
                if let Err(_) = handle.join() {
                    eprintln!("PulseAudio client dispatcher thread panicked.");
                }
            }

            CLIENT_STATE.store(UNINITIALIZED, Ordering::Release);
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
                Facility::Sink if op == subscribe::Operation::New => {
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
    let ListResult::Item(sink) = result else {
        return;
    };

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

fn default_sink_info_callback(result: ListResult<&SinkInfo>) {
    let mut index = DEFAULT_SINK_IDX.write().unwrap();
    match result {
        ListResult::Item(sink) => unsafe {
            HAS_DEFAULT_SINK = true;
            *index = Some(sink.index);
        }
        ListResult::End | ListResult::Error => unsafe {
            if !HAS_DEFAULT_SINK {
                *index = None;
            }

            HAS_DEFAULT_SINK = false;
        }
    }
}

fn cleanup(ctx: Option<&mut Context>) {
    if let Some(ctx) = ctx {
        ctx.set_subscribe_callback(None);
        ctx.set_state_callback(None);
    }

    *SENDER.write().unwrap() = None;
    *RECEIVER.write().unwrap() = None;
    *REQUEST_DISPATCHER.write().unwrap() = None;
}
