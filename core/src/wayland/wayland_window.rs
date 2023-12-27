use smithay_client_toolkit::{
    reexports::{
        calloop_wayland_source::WaylandSource,
        calloop::{
            channel::{self as calloop_channel, channel, Channel},
            EventLoop
        },
        client::{
            globals::{GlobalList, registry_queue_init},
            protocol::{
                wl_output::{WlOutput, Transform},
                wl_shm::Format,
                wl_pointer::WlPointer,
                wl_seat::WlSeat,
                wl_surface::WlSurface
            },
            Connection, QueueHandle
        }
    },
    shell::{wlr_layer::LayerSurface, xdg::popup::Popup},
    compositor::{CompositorHandler, CompositorState},
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    seat::{
        Capability, SeatHandler, SeatState,
        pointer::{PointerEvent, PointerHandler, PointerEventKind},
    },
    shm::{
        slot::{Buffer, SlotPool},
        Shm, ShmHandler,
    },
    delegate_compositor, delegate_output, delegate_registry, delegate_seat,
    delegate_shm, registry_handlers, delegate_pointer
};
use tiny_skia::PixmapMut;
use tokio::{runtime, sync::mpsc::UnboundedSender};

use crate::{
    client::{ClientRequest, MakeUiFn, UiRequest},
    Ui, UiEvent, Event, TaskResult, Point, Size, Theme
};
use super::{WindowEvent, MouseEvent, MouseButton, MouseScrollDelta};

pub(crate) struct WaylandWindowBase<W: WaylandWindow> {
    event_loop: EventLoop<'static, State<W>>,
    queue_handle: QueueHandle<State<W>>,
    state: State<W>
}

pub(crate) trait WaylandWindow: Sized {
    type Config;

    fn init(
        config: Self::Config,
        globals: &GlobalList,
        queue_handle: &QueueHandle<State<Self>>,
        surface: WlSurface
    ) -> (Self, WindowSurface);
}

#[derive(Clone, Debug)]
pub(crate) enum WindowSurface {
    LayerShellSurface(LayerSurface),
    XdgPopup(Popup)
}

pub(crate) struct State<W: WaylandWindow> {
    pub ui: Ui,
    pub window: W,
    pub registry_state: RegistryState,
    pub seat_state: SeatState,
    pub pointer: Option<WlPointer>,
    pub output_state: OutputState,
    pub shm: Shm,
    pub pool: SlotPool,
    pub buffer: Option<Buffer>,
    pub monitor: Monitor,
    pub pending_events: Vec<UiEvent>,
    pub close: bool
}

#[derive(Debug)]
pub(crate) struct Monitor {
    pub output: Option<WlOutput>,
    pub viewport: Viewport
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Viewport {
    pub logical_size: (u32, u32),
    pub scale_factor: f32
}

impl<W: WaylandWindow + 'static> WaylandWindowBase<W> {
    pub fn new(
        config: W::Config,
        client_recv: Channel<ClientRequest>,
        make_ui: MakeUiFn,
        theme: Theme,
        rt_handle: runtime::Handle,
        ui_send: UnboundedSender<UiRequest>
    ) -> Self {
        let conn = Connection::connect_to_env().unwrap();
        let (globals, event_queue) = registry_queue_init(&conn).unwrap();
        let queue_handle: QueueHandle<State<W>> = event_queue.handle();

        let event_loop: EventLoop<State<W>> =
            EventLoop::try_new().expect("Failed to initialize the event loop!");

        let loop_handle = event_loop.handle();
        WaylandSource::new(conn.clone(), event_queue)
            .insert(loop_handle.clone())
            .unwrap();
    
        let compositor_state = CompositorState::bind(&globals, &queue_handle)
            .expect("wl_compositor not available");

        let shm = Shm::bind(&globals, &queue_handle).expect("wl_shm is not available.");
        let pool = SlotPool::new(256 * 256 * 4, &shm)
            .expect("Failed to create a shared memory pool.");

        loop_handle.insert_source(client_recv, |event, _, state| {
            match event  {
                calloop_channel::Event::Msg(request) =>  match request {
                    ClientRequest::Close => state.close = true,
                    ClientRequest::ThemeChanged(theme) =>
                        state.ui.set_theme(theme)
                }
                calloop_channel::Event::Closed => eprintln!("Client channel closed unexpectedly!")
            }
        }).expect("Couldn't register client request source with Wayland event loop.");

        let (task_send, task_recv) = channel::<TaskResult>();
        loop_handle.insert_source(task_recv, |event, _, state| {
            match event  {
                calloop_channel::Event::Msg(result) =>
                    state.pending_events.push(UiEvent::Task(result)),
                calloop_channel::Event::Closed => eprintln!("Task channel closed unexpectedly!")
            }
        }).expect("Couldn't register ui task source with Wayland event loop.");

        let surface = compositor_state.create_surface(&queue_handle);
        let (window, surface) = W::init(config, &globals, &queue_handle, surface);

        Self {
            event_loop,
            state: State {
                ui: make_ui(theme, surface, rt_handle, task_send, ui_send),
                window,
                shm,
                pool,
                buffer: None,
                registry_state: RegistryState::new(&globals),
                seat_state: SeatState::new(&globals, &queue_handle),
                output_state: OutputState::new(&globals, &queue_handle),
                pointer: None,
                pending_events: vec![],
                close: false,
                monitor: Monitor {
                    output: None,
                    viewport: Viewport {
                        logical_size: (256, 256),
                        scale_factor: 1f32
                    }
                }
            },
            queue_handle
        }
    }

    pub fn run(mut self) {
        while !self.state.close {
            if self.state.ui.needs_redraw() {
                self.draw();
            }

            self.event_loop.dispatch(None, &mut self.state).unwrap();

            for event in self.state.pending_events.drain(..) {
                match event {
                    UiEvent::Task(result) => self.state.ui.task_result(result),
                    UiEvent::Window(event) => match event {
                        WindowEvent::ScaleFactor(scale_factor) =>
                            self.state.ui.set_scale_factor(scale_factor),
                        WindowEvent::Resize(size) => {
                            self.state.ui.layout(Size {
                                width: size.0 as f32,
                                height: size.1 as f32
                            });
                        }
                        WindowEvent::Mouse(event) =>
                            self.state.ui.event(Event::Mouse(event))
                    }
                }
            }
        }
    }

    fn draw(&mut self) {
        if self.state.monitor.output.is_none() {
            return;
        }

        let Viewport {
            logical_size,
            scale_factor
        } = self.state.monitor.viewport;

        let scale_factor = scale_factor as u32;
        let (width, height) = (
            logical_size.0 * scale_factor,
            logical_size.1 * scale_factor
        );

        let stride = width as i32 * 4;

        let format = Format::Argb8888;
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

        let mut pixmap = PixmapMut::from_bytes(
            canvas,
            width,
            height
        ).unwrap();

        self.state.ui.draw(&mut pixmap);

        let surface = self.state.ui.wl_surface();
        surface.damage_buffer(0, 0, width as i32, height as i32);
        surface.frame(&self.queue_handle, surface.clone());
        buffer.attach_to(surface).expect("buffer attach");

        surface.commit();
    }
}

impl<W: WaylandWindow> CompositorHandler for State<W> {
    fn scale_factor_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &WlSurface,
        new_factor: i32,
    ) {
        let new_factor = new_factor as f32;
        let viewport = &mut self.monitor.viewport;
        
        if new_factor == viewport.scale_factor {
            return;
        }
        viewport.scale_factor = new_factor;

        self.buffer = None;
        self.ui.wl_surface().set_buffer_scale(new_factor as i32);

        self.pending_events.push(
            UiEvent::Window(WindowEvent::ScaleFactor(new_factor))
        );
    }

    fn frame(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &WlSurface,
        _time: u32,
    ) { }

    fn transform_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &WlSurface,
        _transform: Transform
    ) { }
}

impl<W: WaylandWindow> OutputHandler for State<W> {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }

    fn new_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        output: WlOutput
    ) {
        if let Some(info) = self.output_state.info(&output) {
            println!("{:?}", info);
        }
        self.monitor.output = Some(output);
    }

    fn update_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        output: WlOutput
    ) {
        println!("Update output: {:?}", output);
    }

    fn output_destroyed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        output: WlOutput
    ) {
        if self.monitor.output.as_ref().map_or(false, |x| *x == output) {
            self.monitor.output = None;
        }
    }
}

impl<W: WaylandWindow + 'static> SeatHandler for State<W> {
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

    fn remove_seat(
        &mut self, _: &Connection,
        _: &QueueHandle<Self>,
        _: WlSeat
    ) { }
}

impl<W: WaylandWindow> PointerHandler for State<W> {
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

            let event = match event.kind {
                PointerEventKind::Motion { .. } =>
                        Some(WindowEvent::Mouse(MouseEvent::MouseMove(pos))),
                PointerEventKind::Press { button, .. } =>
                    MouseButton::from_code(button).and_then(|button|
                        Some(WindowEvent::Mouse(MouseEvent::MousePress { button, pos }))
                    ),
                PointerEventKind::Release { button, .. } =>
                    MouseButton::from_code(button).and_then(|button|
                        Some(WindowEvent::Mouse(MouseEvent::MouseRelease { button, pos }))
                    ),
                PointerEventKind::Axis { horizontal, vertical, .. } => {
                    let has_discrete_scroll = horizontal.discrete != 0 || vertical.discrete != 0;
                    let delta = if has_discrete_scroll {
                        MouseScrollDelta::Line {
                            x: horizontal.discrete as f32,
                            y: vertical.discrete as f32
                        }
                    } else {
                        let scale_factor = self.monitor.viewport.scale_factor as f32;
                        MouseScrollDelta::Pixel {
                            x: horizontal.absolute as f32 * scale_factor,
                            y: vertical.absolute as f32 * scale_factor
                        }
                    };

                    Some(WindowEvent::Mouse(MouseEvent::Scroll(delta)))
                }
                PointerEventKind::Enter { .. } =>
                    Some(WindowEvent::Mouse(MouseEvent::EnterWindow)),
                PointerEventKind::Leave { .. } =>
                    Some(WindowEvent::Mouse(MouseEvent::LeaveWindow))
            };

            if let Some(event) = event {
                self.pending_events.push(UiEvent::Window(event));
            }
        }
    }
}

impl<W: WaylandWindow> ShmHandler for State<W> {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm
    }
}

impl<W: WaylandWindow + 'static> ProvidesRegistryState for State<W> {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    
    registry_handlers![OutputState];
}

delegate_compositor!(@<W: WaylandWindow + 'static> State<W>);
delegate_output!(@<W: WaylandWindow + 'static> State<W>);
delegate_seat!(@<W: WaylandWindow + 'static> State<W>);
delegate_pointer!(@<W: WaylandWindow + 'static> State<W>);
delegate_shm!(@<W: WaylandWindow + 'static> State<W>);
delegate_registry!(@<W: WaylandWindow + 'static> State<W>);