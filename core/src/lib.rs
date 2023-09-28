pub mod widget;
mod ui;
mod geometry;
mod theme;
mod renderer;
mod wayland;
mod color;
mod draw;
mod gradient;

pub use ui::*;
pub use geometry::*;
pub use theme::*;
pub use wayland::*;
pub use color::*;
pub use draw::*;
pub use gradient::*;
pub use tokio;
pub use cosmic_text::{Family, Stretch, Style, Weight};

use tiny_skia::PixmapMut;
use tokio::{
    io::unix::AsyncFd,
    sync::mpsc::channel
};

use crate::widget::Element;

pub async fn run<E: Element>(theme: Theme, build: impl FnOnce() -> E) {
    let mut window = BarWindow::new();

    // TODO: It'd be more efficient to process multiple results at once.
    let (tx, mut rx) = channel::<TaskResult>(100);
    let mut ui = Ui::new(tx, theme, build);

    // Wait for the initial resize event
    {
        let mut events = Vec::new();
        while events.is_empty() {
            events = window.events_blocking();
        }

        assert!(events.iter().any(|x|
            matches!(x, WaylandEvent::Resize(_))
        ));

        process_events(&mut ui, events);
        window.canvas(|canvas, size| {
            let mut pixmap = PixmapMut::from_bytes(
                canvas,
                size.0,
                size.1
            ).unwrap();

            ui.draw(&mut pixmap);
        });
    }
    
    loop {
        let wl_fd = AsyncFd::new(window.events_socket()).unwrap();

        tokio::select! {
            biased;

            _ = wl_fd.readable() => {
                drop(wl_fd);
                
                let events = window.read_events();
                process_events(&mut ui, events);

                // We prioritize events from the compositor but
                // check for completed tasks as well so that we can only
                // do a single layout/draw pass.
                if let Ok(result) = rx.try_recv() {
                    ui.task_result(result);
                }
            }
            result = rx.recv() => {
                drop(wl_fd);

                if let Some(result) = result {
                    ui.task_result(result);
                }
            }
        };

        if ui.needs_redraw() {
            window.canvas(|canvas, size| {
                let mut pixmap = PixmapMut::from_bytes(
                    canvas,
                    size.0,
                    size.1
                ).unwrap();

                ui.draw(&mut pixmap);
            });
        }
    }
}

#[inline]
fn process_events(ui: &mut Ui, events: Vec<WaylandEvent>) {
    for event in events {
        match event {
            WaylandEvent::ScaleFactor(scale_factor) =>
                ui.set_scale_factor(scale_factor),
            WaylandEvent::Resize(size) =>
                ui.layout(Size {
                    width: size.0 as f32,
                    height: size.1 as f32
                }),
            WaylandEvent::Mouse(event) =>
                ui.event(Event::Mouse(event))
        }
    }
}