pub use smithay_client_toolkit::reexports::protocols::xdg::shell::client::xdg_positioner::Anchor;

use smithay_client_toolkit::{
    reexports::client::{
        globals::GlobalList,
        protocol::wl_surface::WlSurface,
        Connection, QueueHandle
    },
    shell::xdg::{
        window::{Window as XdgWindow, WindowConfigure, WindowHandler},
        popup::{Popup as SctkPopup, PopupHandler, PopupConfigure, ConfigureKind},
        XdgShell, XdgPositioner
    },
    delegate_xdg_shell, delegate_xdg_popup, delegate_xdg_window
};

use super::{
    wayland_window::{WaylandWindow, WindowSurface, State},
    WindowEvent, WindowDimensions
};
use crate::{ui::{Ui, UiEvent}, Size, Rect};

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Location {
    Cursor,
    WidgetBounds,
    Bounds(Rect)
}

#[derive(Clone, Copy, Debug)]
pub struct Popup {
    pub size: WindowDimensions,
    pub location: Location,
    pub anchor: Anchor
}

pub(crate) struct PopupWindowState {
    popup: SctkPopup
}

#[derive(Debug)]
pub(crate) struct PopupWindowConfig {
    pub parent: WindowSurface,
    pub size: WindowDimensions,
    pub anchor: Anchor,
    pub anchor_rect: Rect
}

impl Popup {
    #[inline]
    pub fn new(
        size: WindowDimensions,
        location: Location,
        anchor: Anchor
    ) -> Self {
        Self { size, location, anchor }
    }
}

impl WaylandWindow for PopupWindowState {
    type Config = PopupWindowConfig;

    fn init(
        mut config: Self::Config,
        globals: &GlobalList,
        queue_handle: &QueueHandle<State<Self>>,
        surface: WlSurface,
        ui: &mut Ui
    ) -> Self {
        let shell = XdgShell::bind(globals, queue_handle)
            .expect("xdg shell is not available");

        let positioner = XdgPositioner::new(&shell).unwrap();

        let size = match config.size {
            WindowDimensions::Fixed(size) => {
                let size = (size.0 as i32, size.1 as i32);
                positioner.set_size(size.0, size.1);

                size
            }
            WindowDimensions::Auto(max) => {
                let size = ui.layout(Size::new(max.0 as f32, max.1 as f32));
                let size = (size.width.round() as i32, size.height.round() as i32);
                positioner.set_size(size.0, size.1);

                size
            }
        };

        positioner.set_anchor(config.anchor);

        let rect = &mut config.anchor_rect;
        let size_halved = (size.0 as f32 / 2f32, size.1 as f32 / 2f32);
        
        // The popup render origin will be at the center of the anchor_rect so compensate for that.
        // The anchor points being considered around the anchor_rect seems more intuitive to me...
        match config.anchor {
            Anchor::Top => rect.y -= size_halved.1,
            Anchor::Bottom => rect.y += size_halved.1,
            Anchor::Left => rect.x -= size_halved.0,
            Anchor::Right => rect.x += size_halved.0,
            Anchor::TopLeft => {
                rect.x -= size_halved.0;
                rect.y -= size_halved.1;
            }
            Anchor::TopRight => {
                rect.x += size_halved.0;
                rect.y -= size_halved.1;
            }
            Anchor::BottomLeft => {
                rect.x -= size_halved.0;
                rect.y += size_halved.1;
            }
            Anchor::BottomRight => {
                rect.x += size_halved.0;
                rect.y += size_halved.1;
            }
            _ => { }
        }

        positioner.set_anchor_rect(
            rect.x.round() as i32,
            rect.y.round() as i32,
            rect.width.round() as i32,
            rect.height.round() as i32
        );

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

        popup.xdg_surface().set_window_geometry(0, 0, size.0, size.1);
        popup.wl_surface().commit();

        Self { popup }
    }

    #[inline]
    fn window_surface(&self) -> WindowSurface {
        WindowSurface::XdgPopup(self.popup.clone())
    }

    #[inline]
    fn wl_surface(&self) -> &WlSurface {
        self.popup.wl_surface()
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
        popup: &SctkPopup,
        config: PopupConfigure
    ) {
        if *popup != self.window.popup {
            return;
        }

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
        popup: &SctkPopup
    ) {
        if *popup == self.window.popup {
            self.close = true;
        }
    }
}

delegate_xdg_shell!(State<PopupWindowState>);
delegate_xdg_window!(State<PopupWindowState>);
delegate_xdg_popup!(State<PopupWindowState>);
