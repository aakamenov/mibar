use std::{
    thread,
    sync::atomic::{AtomicU64, Ordering},
    hash::{Hash, Hasher},
    collections::hash_map::Entry
};

use nohash::{self, IntMap};
use smithay_client_toolkit::reexports::calloop::channel::{Sender, channel};
use tokio::{
    runtime,
    sync::mpsc::{UnboundedSender, unbounded_channel}
};

use crate::{
    wayland::layer_shell_window::LayerShellBase,
    window::Window,
    widget::Element,
    Ui, Theme, TaskResult
};

#[derive(Clone, Copy, PartialOrd, Eq, PartialEq, Debug)]
pub struct WindowId(u64);

pub(crate) struct UiRequest {
    pub id: WindowId,
    pub action: WindowAction
}

pub(crate) enum WindowAction {
    Open {
        window: Window,
        create_ui: Box<dyn FnOnce(
            runtime::Handle,
            Sender<TaskResult>,
            UnboundedSender<UiRequest>
        ) -> Ui + Send>
    },
    Close
}

#[derive(Debug)]
pub(crate) enum ClientRequest {
    Close
}

pub fn run(
    mut builder: runtime::Builder,
    window: impl Into<Window>,
    root: impl Element + Send + 'static,
    theme: Theme,
    // TODO: Temporary hack to init sysinfo
    on_init: impl FnOnce(&runtime::Handle)
) {
    let runtime = builder.build().unwrap();

    let mut windows = IntMap::default();
    let (ui_send, mut ui_recv) = unbounded_channel::<UiRequest>();

    let id = WindowId::new();
    let create_ui = Box::new(move |rt_handle, task_send, client_send| {
        Ui::new(id, rt_handle, task_send, client_send, theme, root)
    });

    ui_send.send(UiRequest {
        id,
        action: WindowAction::Open {
            window: window.into(),
            create_ui
        }
    }).unwrap();

    runtime.block_on(async {
        on_init(runtime.handle());

        while let Some(request) = ui_recv.recv().await {
            match request.action {
                WindowAction::Open { window, create_ui } => {
                    let (client_send, client_recv) = channel::<ClientRequest>();
                    windows.insert(request.id, client_send);

                    let rt_handle = runtime.handle().clone();
                    let ui_send = ui_send.clone();
                    thread::spawn(move || {
                        let (task_send, task_recv) = channel::<TaskResult>();
                        let ui = create_ui(rt_handle, task_send, ui_send);

                        match window {
                            Window::Bar(bar) => {
                                LayerShellBase::new(
                                    bar,
                                    ui,
                                    client_recv,
                                    task_recv
                                ).run();
                            }
                            Window::Panel(panel) => {
                                LayerShellBase::new(
                                    panel,
                                    ui,
                                    client_recv,
                                    task_recv
                                ).run();
                            }
                        }
                    });
                },
                WindowAction::Close => {
                    if let Entry::Occupied(entry) = windows.entry(request.id) {
                        if entry.get().send(ClientRequest::Close).is_err() {
                            entry.remove();
                        }
                    }

                    if windows.is_empty() {
                        break;
                    }
                }
            }
        }
    });

    // Force shutdown any tasks that are still running.
    runtime.shutdown_background();
}

impl WindowId {
    pub(crate) fn new() -> Self {
        static WINDOW_ID: AtomicU64 = AtomicU64::new(1);

        Self(WINDOW_ID.fetch_add(1, Ordering::Relaxed))
    }
}

impl Hash for WindowId {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u64(self.0);
    }
}

impl nohash::IsEnabled for WindowId { }
