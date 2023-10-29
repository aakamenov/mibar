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
    WindowEvent
};
use crate::ui::UiEvent;

pub struct LayerShellWindowState {
    surface: LayerSurface,
    desired_size: (u32, u32)
}

pub struct LayerShellWindowConfig {
    pub anchor: Anchor,
    pub layer: Layer,
    pub desired_size: (u32, u32),
    pub exclusive_zone: Option<i32>
}

impl WaylandWindow for LayerShellWindowState {
    type Config = LayerShellWindowConfig;

    fn init(
        config: Self::Config,
        globals: &GlobalList,
        queue_handle: &QueueHandle<State<Self>>,
        surface: WlSurface
    ) -> Self {
        let layer_shell = LayerShell::bind(&globals, &queue_handle)
            .expect("Compositor does not support the zwlr_layer_shell_v1 protocol.");

        let surface = layer_shell.create_layer_surface(
            &queue_handle,
            surface,
            config.layer,
            Some("mibar"),
            None
        );

        surface.set_size(config.desired_size.0, config.desired_size.1);
        surface.set_anchor(config.anchor);
        surface.commit();

        LayerShellWindowState {
            surface,
            desired_size: config.desired_size
        }
    }

    #[inline]
    fn wl_surface(&self) -> &WlSurface {
        self.surface.wl_surface()
    }
    
    #[inline]
    fn as_window_surface(&self) -> WindowSurface {
        WindowSurface::LayerShellSurface(self.surface.clone())
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
        layer: &LayerSurface,
        configure: LayerSurfaceConfigure,
        _serial: u32,
    ) {
        assert_eq!(self.window.surface, *layer);

        let size = if configure.new_size.0 == 0 || configure.new_size.1 == 0 {
            self.window.desired_size
        } else {
            configure.new_size
        };

        if self.monitor.viewport.logical_size != size {
            self.monitor.viewport.logical_size = size;
            self.window.surface.set_size(size.0, size.1);
            self.window.surface.commit();

            self.buffer = None;
            self.pending_events.push(UiEvent::Window(WindowEvent::Resize(size)));
        }
    }
}

delegate_layer!(State<LayerShellWindowState>);
