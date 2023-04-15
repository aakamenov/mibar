mod ui;
mod geometry;
mod widget;
mod theme;
mod positioner;

use smithay_client_toolkit::{
    reexports::client::{
        globals::registry_queue_init,
        protocol::{wl_output, wl_seat, wl_surface, wl_shm},
        Connection, QueueHandle,
    },
    compositor::{CompositorHandler, CompositorState},
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    seat::{Capability, SeatHandler, SeatState},
    shell::{
        wlr_layer::{
            LayerShellHandler, LayerShell, LayerSurface,
            LayerSurfaceConfigure, Layer, Anchor
        },
        WaylandSurface
    },
    shm::{
        slot::{Buffer, SlotPool},
        Shm, ShmHandler,
    },
    delegate_compositor, delegate_output, delegate_registry, delegate_seat,
    delegate_xdg_shell, delegate_layer, delegate_shm, registry_handlers
};
use tiny_skia::PixmapMut;

use crate::{
    ui::Ui,
    widget::bar::Bar,
    geometry::Size
};

fn main() {
    let conn = Connection::connect_to_env().unwrap();
    let (globals, mut event_queue) = registry_queue_init(&conn).unwrap();
    let qh = event_queue.handle();

    // Initialize xdg_shell handlers so we can select the correct adapter
    let compositor_state =
        CompositorState::bind(&globals, &qh).expect("wl_compositor not available");

    let layer_shell = LayerShell::bind(&globals, &qh)
        .expect("Compositor does not support the zwlr_layer_shell_v1 protocol.");

    let surface = compositor_state.create_surface(&qh);

    let shm = Shm::bind(&globals, &qh).expect("wl_shm is not available.");

    let layer_surface = layer_shell.create_layer_surface(
        &qh,
        surface,
        Layer::Top,
        Option::<String>::None,
        None
    );

    let pool = SlotPool::new(256 * 256 * 4, &shm)
        .expect("Failed to create a shared memory pool.");

    let mut bar = Mibar {
        registry_state: RegistryState::new(&globals),
        seat_state: SeatState::new(&globals, &qh),
        output_state: OutputState::new(&globals, &qh),
        shm,
        pool,
        buffer: None,
        layer_shell,
        layer_surface,
        exit: false,
        width: 256,
        height: 256,
        ui: Ui::new(Box::new(Bar::new()))
    };

    // We don't draw immediately, the configure will notify us when to first draw.
    loop {
        event_queue.blocking_dispatch(&mut bar).unwrap();

        if bar.exit {
            println!("exiting example");
            break;
        }
    }
}

struct Mibar {
    registry_state: RegistryState,
    seat_state: SeatState,
    output_state: OutputState,
    layer_surface: LayerSurface,
    layer_shell: LayerShell,
    pool: SlotPool,
    buffer: Option<Buffer>,
    shm: Shm,
    exit: bool,
    width: u32,
    height: u32,
    ui: Ui
}

impl CompositorHandler for Mibar {
    fn scale_factor_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _new_factor: i32,
    ) {
    }

    fn frame(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _time: u32,
    ) {
    }
}

impl OutputHandler for Mibar {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }

    fn new_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        output: wl_output::WlOutput,
    ) {
        let Some(info) = self.output_state.info(&output) else {
            return;
        };

        let Some(size) = info.logical_size else {
            return;
        };
        println!("New output: {:?}", info);

        self.layer_surface.set_anchor(Anchor::BOTTOM);
        self.layer_surface.set_size(size.0 as u32, 40);
        self.layer_surface.set_exclusive_zone(40);
        self.layer_surface.commit();
    }

    fn update_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        output: wl_output::WlOutput,
    ) {
        println!("Update output: {:?}", output);
    }

    fn output_destroyed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        output: wl_output::WlOutput,
    ) {
        println!("Output destroyed: {:?}", output);
    }
}

impl SeatHandler for Mibar {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.seat_state
    }

    fn new_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}

    fn new_capability(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _seat: wl_seat::WlSeat,
        _capability: Capability,
    ) {
    }

    fn remove_capability(
        &mut self,
        _conn: &Connection,
        _: &QueueHandle<Self>,
        _: wl_seat::WlSeat,
        _capability: Capability,
    ) {
    }

    fn remove_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}
}

impl ShmHandler for Mibar {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm
    }
}

impl ProvidesRegistryState for Mibar {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    
    registry_handlers![OutputState];
}

impl LayerShellHandler for Mibar {
    fn closed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _layer: &LayerSurface
    ) {
    }

    fn configure(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        _layer: &LayerSurface,
        configure: LayerSurfaceConfigure,
        _serial: u32,
    ) {
        println!("layer draw size: {:?}", configure.new_size);
        self.width = configure.new_size.0;
        self.height = configure.new_size.1;

        self.ui.layout(Size {
            width: self.width as f32,
            height: self.height as f32
        });
        self.draw(qh);
    }
}

impl Mibar {
    fn draw(&mut self, qh: &QueueHandle<Self>) {
        println!("redraw");

        let width = self.width;
        let height = self.height;
        let stride = self.width as i32 * 4;

        let format = wl_shm::Format::Argb8888;

        let buffer = self.buffer.get_or_insert_with(|| {
            self.pool
                .create_buffer(width as i32, height as i32, stride, format)
                .expect("create buffer")
                .0
        });

        let canvas = match self.pool.canvas(buffer) {
            Some(canvas) => canvas,
            None => {
                // This should be rare, but if the compositor has not released the previous
                // buffer, we need double-buffering.
                let (second_buffer, canvas) = self.pool
                    .create_buffer(
                        width as i32,
                        height as i32,
                        stride,
                        format
                    ).expect("create buffer");

                *buffer = second_buffer;

                canvas
            }
        };

        let mut pixmap = PixmapMut::from_bytes(canvas, width, height).unwrap();
        self.ui.draw(&mut pixmap);        

        let surface = self.layer_surface.wl_surface();
        // Damage the entire window
        surface.damage_buffer(0, 0, self.width as i32, self.height as i32);

        // Request our next frame
        surface.frame(qh, surface.clone());

        // Attach and commit to present.
        buffer.attach_to(surface).expect("buffer attach");

        self.layer_surface.commit();
    }
}

delegate_compositor!(Mibar);
delegate_output!(Mibar);

delegate_seat!(Mibar);

delegate_xdg_shell!(Mibar);
delegate_shm!(Mibar);

delegate_registry!(Mibar);

delegate_layer!(Mibar);
