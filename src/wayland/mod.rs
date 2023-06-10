use std::{
    mem,
    os::fd::BorrowedFd
};

use smithay_client_toolkit::{
    reexports::client::{
        globals::registry_queue_init,
        protocol::{
            wl_output, wl_surface, wl_shm,
            wl_pointer::WlPointer,
            wl_seat::WlSeat
        },
        backend::ReadEventsGuard,
        Connection, QueueHandle, EventQueue
    },
    compositor::{CompositorHandler, CompositorState},
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    seat::{
        Capability, SeatHandler, SeatState,
        pointer::{PointerEvent, PointerHandler, PointerEventKind},
    },
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
    delegate_xdg_shell, delegate_layer, delegate_shm, registry_handlers,
    delegate_pointer
};

use crate::geometry::Point;

#[derive(Debug)]
pub enum WaylandEvent {
    Resize((u32, u32)),
    Mouse(MouseEvent)
}

#[derive(Clone, Copy, Debug)]
pub enum MouseEvent {
    MousePress {
        pos: Point,
        button: MouseButton
    },
    MouseRelease {
        pos: Point,
        button: MouseButton
    },
    MouseMove(Point)
}

#[derive(Clone, Copy, Debug)]
pub enum MouseButton {
    Left,
    Right,
    Middle
}

pub struct BarWindow {
    event_queue: EventQueue<State>,
    state: State,
    read_events_guard: Option<ReadEventsGuard>
}

struct State {
    registry_state: RegistryState,
    seat_state: SeatState,
    pointer: Option<WlPointer>,
    output_state: OutputState,
    layer_surface: LayerSurface,
    layer_shell: LayerShell,
    pool: SlotPool,
    buffer: Option<Buffer>,
    shm: Shm,
    size: (u32, u32),
    pending_events: Vec<WaylandEvent>
}

impl BarWindow {
    pub fn new() -> Self {
        let conn = Connection::connect_to_env().unwrap();
        let (globals, event_queue) = registry_queue_init(&conn).unwrap();
        let qh: QueueHandle<State> = event_queue.handle();
    
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

        Self {
            event_queue,
            read_events_guard: None,
            state: State {
                registry_state: RegistryState::new(&globals),
                seat_state: SeatState::new(&globals, &qh),
                pointer: None,
                output_state: OutputState::new(&globals, &qh),
                shm,
                pool,
                buffer: None,
                layer_shell,
                layer_surface,
                size: (256, 256),
                pending_events: vec![]
            }
        }
    }

    pub fn events_socket(&mut self) -> BorrowedFd {
        // flush the outgoing buffers to ensure that
        // the server receives the messages
        self.event_queue.flush().unwrap();

        let read_guard = self.event_queue.prepare_read().unwrap();
        self.read_events_guard = Some(read_guard);

        self.read_events_guard.as_ref().unwrap().connection_fd()
    }

    pub fn read_events(&mut self) -> Vec<WaylandEvent> {
        let guard = self.read_events_guard.take()
            .expect("Call events_socket() first.");

        guard.read().unwrap();
        self.event_queue.dispatch_pending(&mut self.state).unwrap();

        mem::take(&mut self.state.pending_events)
    }

    pub fn canvas(
        &mut self,
        func: impl FnOnce(&mut [u8], (u32, u32))
    ) {
        let width = self.state.size.0;
        let height = self.state.size.1;
        let stride = width as i32 * 4;

        let format = wl_shm::Format::Argb8888;

        let buffer = self.state.buffer.get_or_insert_with(|| {
            self.state.pool
                .create_buffer(width as i32, height as i32, stride, format)
                .expect("create buffer")
                .0
        });

        let canvas = match self.state.pool.canvas(buffer) {
            Some(canvas) => canvas,
            None => {
                // This should be rare, but if the compositor has not released the previous
                // buffer, we need double-buffering.
                let (second_buffer, canvas) = self.state.pool
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

        func(canvas, self.state.size);

        let surface = self.state.layer_surface.wl_surface();

        // Damage the entire window
        surface.damage_buffer(0, 0, width as i32, height as i32);

        // Request our next frame
        surface.frame(&self.event_queue.handle(), surface.clone());

        // Attach and commit to present.
        buffer.attach_to(surface).expect("buffer attach");

        self.state.layer_surface.commit();
    }
}

impl CompositorHandler for State {
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

impl OutputHandler for State {
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

impl ShmHandler for State {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm
    }
}

impl LayerShellHandler for State {
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
        _qh: &QueueHandle<Self>,
        _layer: &LayerSurface,
        configure: LayerSurfaceConfigure,
        _serial: u32,
    ) {
        // If the width or height arguments are zero, it means the
        // client should decide its own window dimension.
        // The size is a hint, in the sense that the client is free to ignore it
        // if it doesn't resize or pick a smaller size.
        //
        // Therefore, we only care when a bigger size has been assigned.
        // But it seems that this can't even happen in our case. 
        if configure.new_size.0 <= self.size.0 &&
            configure.new_size.1 <= self.size.1 {
            return;
        }

        self.size = configure.new_size;
        self.pending_events.push(WaylandEvent::Resize(self.size)); 
    }
}

impl SeatHandler for State {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.seat_state
    }

    fn new_seat(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        seat: WlSeat
    ) {
        if let Some(info) = self.seat_state.info(&seat) {
            println!("New seat: {}", info);
        }
    }

    fn new_capability(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        seat: WlSeat,
        capability: Capability
    ) {
        println!("New Capability: {}", capability);

        match capability {
            Capability::Pointer if self.pointer.is_none() => {
                if let Ok(pointer) = self.seat_state.get_pointer(qh, &seat) {
                    self.pointer = Some(pointer);
                }
            },
            _ => { }
        }
    }

    fn remove_capability(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _seat: WlSeat,
        capability: Capability,
    ) {
        match capability {
            Capability::Pointer if self.pointer.is_some() => {
                self.pointer.take().unwrap().release();
            }
            _ => { }
        }
    }

    fn remove_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: WlSeat) {}
}

impl PointerHandler for State {
    fn pointer_frame(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _pointer: &WlPointer,
        events: &[PointerEvent]
    ) {
        for event in events {
            let pos = Point::new(
                event.position.0 as f32,
                event.position.1 as f32
            );

            match event.kind {
                PointerEventKind::Motion { .. } =>
                    self.pending_events.push(
                        WaylandEvent::Mouse(
                            MouseEvent::MouseMove(pos)
                        )
                    ),
                PointerEventKind::Press { button, ..} =>
                    if let Some(button) = MouseButton::from_code(button) {
                        self.pending_events.push(
                            WaylandEvent::Mouse(
                                MouseEvent::MousePress {
                                    button,
                                    pos
                                }
                            )
                        );
                    }
                PointerEventKind::Release { button, .. } =>
                    if let Some(button) = MouseButton::from_code(button) {
                        self.pending_events.push(
                            WaylandEvent::Mouse(
                                MouseEvent::MouseRelease {
                                    button,
                                    pos
                                }
                            )
                        );
                    }
                _ => { }
            }
        }
    }
}

impl ProvidesRegistryState for State {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    
    registry_handlers![OutputState];
}

delegate_compositor!(State);
delegate_output!(State);
delegate_seat!(State);
delegate_pointer!(State);
delegate_xdg_shell!(State);
delegate_shm!(State);
delegate_registry!(State);
delegate_layer!(State);

impl MouseButton {
    #[inline]
    fn from_code(code: u32) -> Option<Self> {
        match code {
            272 => Some(MouseButton::Left),
            273 => Some(MouseButton::Right),
            274 => Some(MouseButton::Middle),
            _ => None
        }
    }
}
