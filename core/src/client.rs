use std::{
    thread,
    sync::RwLock,
    hash::{Hash, Hasher},
    collections::hash_map::Entry
};

use nohash::{self, IntMap};
use smithay_client_toolkit::reexports::{
    calloop::channel::{Sender, channel},
    client::Connection
};
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

static WINDOWS: RwLock<Vec<WindowInfo>> = RwLock::new(Vec::new());

pub(crate) type MakeUiFn = 
    Box<dyn FnOnce(
        Theme,
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

struct WindowInfo {
    id: WindowId,
    surface: Option<WindowSurface>
}

pub fn run<E: Element>(
    mut builder: runtime::Builder,
    window: impl Into<Window>,
    root: impl FnOnce() -> E + Send + 'static,
    mut theme: Theme
) {
    let conn = Connection::connect_to_env().unwrap();
    let runtime = builder.build().unwrap();

    let mut windows = IntMap::default();
    let (ui_send, mut ui_recv) = unbounded_channel::<UiRequest>();

    let id = WindowId::new();
    let make_ui = Box::new(move |theme, rt_handle, task_send, client_send| {
        let root = root();
        let ui = Ui::new(id, rt_handle, task_send, client_send, theme, root);

        ui
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
                    let conn = conn.clone();

                    thread::spawn(move || {
                        match config {
                            WindowConfig::LayerShell(bar) => {
                                WaylandWindowBase::<LayerShellWindowState>::new(
                                    bar.into(),
                                    client_recv,
                                    make_ui,
                                    theme,
                                    rt_handle,
                                    ui_send,
                                    conn
                                ).run();
                            }
                            WindowConfig::Popup(popup) => {
                                WaylandWindowBase::<PopupWindowState>::new(
                                    popup,
                                    client_recv,
                                    make_ui,
                                    theme,
                                    rt_handle,
                                    ui_send,
                                    conn
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
        static mut WINDOW_ID: u64 = 0;

        let mut windows = WINDOWS.write().unwrap();

        // Safety: Dereferencing WINDOW_ID happens while we are holding the write lock.
        unsafe {
            let id = Self(WINDOW_ID);
            WINDOW_ID += 1;

            windows.push(WindowInfo { id, surface: None });

            id
        }
    }

    #[inline]
    pub fn is_alive(&self) -> bool {
        WINDOWS.read().unwrap().iter().find(|x| x.id == *self).is_some()
    }

    #[inline]
    pub(crate) fn surface(&self) -> Option<WindowSurface> {
        WINDOWS.read().unwrap().iter()
            .find(|x| x.id == *self)
            .map(|x| x.surface.clone())
            .flatten()
    }

    #[inline]
    pub(crate) fn set_surface(&self, surface: WindowSurface) {
        let mut lock = WINDOWS.write().unwrap();
        let info = lock.iter_mut().find(|x| x.id == *self).unwrap();
        info.surface = Some(surface);
    }

    pub(crate) fn kill(&self) {
        let mut windows = WINDOWS.write().unwrap();
        
        if let Some(index) = windows.iter().position(|x| x.id == *self) {
            windows.swap_remove(index);
        }
    }
}

impl Hash for WindowId {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u64(self.0);
    }
}

impl nohash::IsEnabled for WindowId { }
