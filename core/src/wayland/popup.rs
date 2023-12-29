use smithay_client_toolkit::{
    reexports::{
        protocols::xdg::shell::client::xdg_positioner::Anchor,
        client::{
            globals::GlobalList,
            protocol::wl_surface::WlSurface,
            Connection, QueueHandle
        }
    },
    shell::xdg::{
        window::{Window as XdgWindow, WindowConfigure, WindowHandler},
        popup::{Popup as SctkPopup, PopupHandler, PopupConfigure, ConfigureKind},
        XdgShell, XdgPositioner
    },
    delegate_xdg_shell, delegate_xdg_popup, delegate_xdg_window
};

use super::{
    wayland_window::{State, WaylandWindow, WindowSurface},
    WindowEvent
};
use crate::ui::UiEvent;

#[derive(Clone, Copy, Debug)]
pub struct Popup {
    pub size: (u32, u32)
}

pub(crate) struct PopupWindowState {
    popup: SctkPopup,
}

#[derive(Debug)]
pub(crate) struct PopupWindowConfig {
    pub parent: WindowSurface,
    pub size: (u32, u32)
}

impl Popup {
    #[inline]
    pub fn new(size: (u32, u32)) -> Self {
        Self { size }
    }
}

impl WaylandWindow for PopupWindowState {
    type Config = PopupWindowConfig;

    fn init(
        config: Self::Config,
        globals: &GlobalList,
        queue_handle: &QueueHandle<State<Self>>,
        surface: WlSurface
    ) -> (Self, WindowSurface) {
        let shell = XdgShell::bind(globals, queue_handle)
            .expect("xdg shell is not available");

        let positioner = XdgPositioner::new(&shell).unwrap();
        positioner.set_size(config.size.0 as i32, config.size.1 as i32);
        positioner.set_offset(0, 0);
        positioner.set_anchor(Anchor::BottomLeft);

        let popup = match config.parent {
            WindowSurface::LayerShellSurface(parent) => {
                let popup = SctkPopup::from_surface(
                    None,
                    &positioner,
                    queue_handle,
                    surface,
                    &shell
                ).unwrap();

                parent.get_popup(popup.xdg_popup());

                popup
            }
            WindowSurface::XdgPopup(parent) =>
                SctkPopup::from_surface(
                    Some(parent.xdg_shell_surface().xdg_surface()),
                    &positioner,
                    queue_handle,
                    surface,
                    &shell
                ).unwrap()
        };

        popup.wl_surface().commit();

        let popup_clone = popup.clone();

        (Self { popup }, WindowSurface::XdgPopup(popup_clone))
    }
}

impl WindowHandler for State<PopupWindowState> {
    fn request_close(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _window: &XdgWindow
    ) {
        println!("xdg window close requested");
    }

    fn configure(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _window: &XdgWindow,
        configure: WindowConfigure,
        _serial: u32
    ) {
        println!("xdg window configure {:?}", configure);
    }
}

impl PopupHandler for State<PopupWindowState> {
    fn configure(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _popup: &SctkPopup,
        config: PopupConfigure
    ) {
        if !matches!(config.kind, ConfigureKind::Initial) {
            return;
        }

        let size = (config.width as u32, config.height as u32);
        self.pending_events.push(UiEvent::Window(WindowEvent::Resize(size)));
    }

    fn done(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _popup: &SctkPopup
    ) {
        println!("closing popup");
        self.close = true;
    }
}

delegate_xdg_shell!(State<PopupWindowState>);
delegate_xdg_window!(State<PopupWindowState>);
delegate_xdg_popup!(State<PopupWindowState>);
