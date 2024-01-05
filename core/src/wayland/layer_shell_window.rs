use smithay_client_toolkit::{
    reexports::client::{
        globals::GlobalList,
        protocol::wl_surface::WlSurface,
        Connection, QueueHandle
    },
    shell::{
        wlr_layer::{
            LayerShell, LayerSurface, Layer,  Anchor,
            LayerShellHandler, LayerSurfaceConfigure
         },
        WaylandSurface
    },
    delegate_layer
};

use super::{
    wayland_window::{WaylandWindow, State, WindowSurface},
    WindowEvent, WindowDimensions
};
use crate::{ui::{Ui, UiEvent}, Size};

pub struct LayerShellWindowState {
    surface: LayerSurface
}

#[derive(Debug)]
pub struct LayerShellWindowConfig {
    pub anchor: Anchor,
    pub layer: Layer,
    pub size: WindowDimensions,
    pub exclusive_zone: Option<i32>
}

impl WaylandWindow for LayerShellWindowState {
    type Config = LayerShellWindowConfig;

    fn init(
        config: Self::Config,
        globals: &GlobalList,
        queue_handle: &QueueHandle<State<Self>>,
        surface: WlSurface,
        ui: &mut Ui
    ) -> (Self, WindowSurface) {
        let layer_shell = LayerShell::bind(&globals, &queue_handle)
            .expect("Compositor does not support the zwlr_layer_shell_v1 protocol.");

        let surface = layer_shell.create_layer_surface(
            &queue_handle,
            surface,
            config.layer,
            Some("mibar"),
            None
        );

        surface.set_anchor(config.anchor);

        match config.size {
            WindowDimensions::Fixed(size) => surface.set_size(size.0, size.1),
            WindowDimensions::Auto(max) => {
                let size = ui.layout(Size::new(max.0 as f32, max.1 as f32));
                surface.set_size(size.width.round() as u32, size.height.round() as u32);
            }
        }

        if let Some(zone) = config.exclusive_zone {
            surface.set_exclusive_zone(zone);
        }

        surface.commit();

        let state = LayerShellWindowState {
            surface: surface.clone()
        };

        (state, WindowSurface::LayerShellSurface(surface))
    }
}

impl LayerShellHandler for State<LayerShellWindowState> {
    fn closed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _layer: &LayerSurface
    ) {
        self.close = true;
    }

    fn configure(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _layer: &LayerSurface,
        configure: LayerSurfaceConfigure,
        _serial: u32,
    ) {
        let current_size = self.monitor.surface_info.logical_size;
        let width = if configure.new_size.0 == 0 {
            current_size.0
        } else {
            configure.new_size.0
        };

        let height = if configure.new_size.1 == 0 {
            current_size.1
        } else {
            configure.new_size.1
        };

        let new_size = (width, height);

        if current_size != new_size {
            self.monitor.surface_info.logical_size = new_size;
            self.window.surface.set_size(new_size.0, new_size.1);
            self.window.surface.commit();

            self.buffer = None;
            self.pending_events.push(UiEvent::Window(WindowEvent::Resize(new_size)));
        }
    }
}

delegate_layer!(State<LayerShellWindowState>);
