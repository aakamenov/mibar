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
    wayland::{
        wayland_window::WaylandWindowBase,
        layer_shell_window::LayerShellWindowState,
    },
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
    Close,
    ThemeChanged(Theme)
}

#[derive(Debug)]
pub(crate) enum ClientRequest {
    Close,
    ThemeChanged(Theme)
}

pub fn run(
    mut builder: runtime::Builder,
    window: impl Into<Window>,
    root: impl Element + Send + 'static,
    mut theme: Theme,
    // TODO: Temporary hack to init sysinfo
    on_init: impl FnOnce(&runtime::Handle)
) {
    let runtime = builder.build().unwrap();

    let mut windows = IntMap::default();
    let (ui_send, mut ui_recv) = unbounded_channel::<UiRequest>();

    let id = WindowId::new();
    let theme_clone = theme.clone();
    let create_ui = Box::new(move |rt_handle, task_send, client_send| {
        Ui::new(id, rt_handle, task_send, client_send, theme_clone, root)
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
                                WaylandWindowBase::<LayerShellWindowState>::new(
                                    bar.into(),
                                    ui,
                                    client_recv,
                                    task_recv
                                ).run();
                            }
                            Window::SidePanel(panel) => {
                                WaylandWindowBase::<LayerShellWindowState>::new(
                                    panel.into(),
                                    ui,
                                    client_recv,
                                    task_recv
                                ).run();
                            }
                        }
                    });
                }
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
                WindowAction::ThemeChanged(new_theme) => {
                    for (id, sender) in &windows {
                        if *id == request.id {
                            continue;
                        }

                        if sender.send(
                            ClientRequest::ThemeChanged(new_theme.clone())
                        ).is_err() {
                            eprintln!("Child UI thread has terminated unexpectedly!");
                        }
                    }

                    theme = new_theme;
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
