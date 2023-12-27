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
        wayland_window::{WaylandWindowBase, WindowSurface},
        layer_shell_window::LayerShellWindowState,
        popup::PopupWindowState,
        WindowConfig
    },
    window::Window,
    widget::Element,
    Ui, Theme, TaskResult
};

pub(crate) type MakeUiFn = 
    Box<dyn FnOnce(
        Theme,
        WindowSurface,
        runtime::Handle,
        Sender<TaskResult>,
        UnboundedSender<UiRequest>
    ) -> Ui + Send>;

#[derive(Clone, Copy, PartialOrd, Eq, PartialEq, Debug)]
pub struct WindowId(u64);

pub(crate) struct UiRequest {
    pub id: WindowId,
    pub action: WindowAction
}

pub(crate) enum WindowAction {
    Open {
        config: WindowConfig,
        make_ui: MakeUiFn
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
    mut theme: Theme
) {
    let runtime = builder.build().unwrap();

    let mut windows = IntMap::default();
    let (ui_send, mut ui_recv) = unbounded_channel::<UiRequest>();

    let id = WindowId::new();
    let make_ui = Box::new(move |theme, surface, rt_handle, task_send, client_send| {
        Ui::new(id, surface, rt_handle, task_send, client_send, theme, root)
    });

    ui_send.send(UiRequest {
        id,
        action: WindowAction::Open {
            config: match window.into() {
                Window::Bar(bar) => WindowConfig::LayerShell(bar.into()),
                Window::SidePanel(panel) => WindowConfig::LayerShell(panel.into()),
                Window::Popup(_) => panic!("Initial window cannot be a popup.")
            },
            make_ui
        }
    }).unwrap();

    runtime.block_on(async {
        while let Some(request) = ui_recv.recv().await {
            match request.action {
                WindowAction::Open { config, make_ui } => {
                    let (client_send, client_recv) = channel::<ClientRequest>();
                    windows.insert(request.id, client_send);

                    let rt_handle = runtime.handle().clone();
                    let ui_send = ui_send.clone();
                    let theme = theme.clone();

                    thread::spawn(move || {
                        match config {
                            WindowConfig::LayerShell(bar) => {
                                WaylandWindowBase::<LayerShellWindowState>::new(
                                    bar.into(),
                                    client_recv,
                                    make_ui,
                                    theme,
                                    rt_handle,
                                    ui_send
                                ).run();
                            }
                            WindowConfig::Popup(popup) => {
                                WaylandWindowBase::<PopupWindowState>::new(
                                    popup,
                                    client_recv,
                                    make_ui,
                                    theme,
                                    rt_handle,
                                    ui_send
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