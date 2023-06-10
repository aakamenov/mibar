mod ui;
mod geometry;
mod widget;
mod theme;
mod renderer;
mod bar;
mod wayland;

use tiny_skia::PixmapMut;
use tokio::{
    io::unix::AsyncFd,
    sync::mpsc::channel
};

use crate::{
    ui::{Ui, Event, TaskResult},
    geometry::Size,
    wayland::{BarWindow, WaylandEvent}
};

#[tokio::main(flavor = "multi_thread", worker_threads = 10)]
async fn main() {
    let mut window = BarWindow::new();

    // TODO: It'd be more efficient to process multiple results at once.
    let (tx, mut rx) = channel::<TaskResult>(100);
    let mut ui = Ui::new(tx, bar::build);
    
    loop {
        let wl_fd = AsyncFd::new(window.events_socket()).unwrap();

        tokio::select! {
            biased;

            _ = wl_fd.readable() => {
                drop(wl_fd);

                for event in window.read_events() {
                    match event {
                        WaylandEvent::Resize(size) =>
                            ui.layout(Size {
                                width: size.0 as f32,
                                height: size.1 as f32
                            }),
                        WaylandEvent::Mouse(event) =>
                            ui.event(Event::Mouse(event))
                    }
                }

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
