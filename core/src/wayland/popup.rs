use smithay_client_toolkit::{
    reexports::client::{
        globals::GlobalList,
        protocol::wl_surface::WlSurface,
        Connection, QueueHandle
    },
    shell::{
        xdg::{
            window::{WindowHandler, Window as XdgWindow},
            popup::{Popup, PopupConfigure, PopupHandler},
            XdgShell, XdgPositioner
        },
    },
    delegate_xdg_shell, delegate_xdg_popup, delegate_xdg_window
};

use super::wayland_window::{State, WaylandWindow, WindowSurface};

pub(crate) struct PopupWindow {
    popup: Popup
}

pub(crate) struct PopupWindowConfig {
    parent: WindowSurface
}

impl WaylandWindow for PopupWindow {
    type Config = PopupWindowConfig;

    fn init(
        config: Self::Config,
        globals: &GlobalList,
        queue_handle: &QueueHandle<State<Self>>,
        surface: WlSurface
    ) -> Self {
        let shell = XdgShell::bind(globals, queue_handle)
            .expect("xdg shell is not available");

        let positioner = XdgPositioner::new(&shell).unwrap();
        let popup = Popup::from_surface(
            None,
            &positioner,
            queue_handle,
            surface.clone(),
            &shell
        ).unwrap();

        let popup = match config.parent {
            WindowSurface::LayerShellSurface(parent) => {
                let popup = Popup::from_surface(
                    None,
                    &positioner,
                    queue_handle,
                    surface,
                    &shell
                ).unwrap();

                parent.get_popup(popup.xdg_popup());

                popup
            }
            WindowSurface::XdgShellSurface(parent) =>
                Popup::from_surface(
                    Some(&parent),
                    &positioner,
                    queue_handle,
                    surface,
                    &shell
                ).unwrap()
        };

        Self { popup }
    }

    #[inline]
    fn wl_surface(&self) -> &WlSurface {
        self.popup.wl_surface()
    }

    #[inline]
    fn as_window_surface(&self) -> WindowSurface {
        WindowSurface::XdgShellSurface(self.popup.xdg_surface().clone())
    }
}

impl WindowHandler for State<PopupWindow> {
    fn request_close(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        window: &XdgWindow
    ) {
        todo!()
    }

    fn configure(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        window: &smithay_client_toolkit::shell::xdg::window::Window,
        configure: smithay_client_toolkit::shell::xdg::window::WindowConfigure,
        serial: u32,
    ) {
        todo!()
    }
}

impl PopupHandler for State<PopupWindow> {
    fn configure(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        popup: &Popup,
        config: PopupConfigure,
    ) {
        todo!()
    }

    fn done(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        popup: &Popup
    ) {
        todo!()
    }
}

delegate_xdg_shell!(State<PopupWindow>);
delegate_xdg_window!(State<PopupWindow>);
delegate_xdg_popup!(State<PopupWindow>);
