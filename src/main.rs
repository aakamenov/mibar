mod ui;
mod geometry;
mod widget;
mod theme;
mod renderer;
mod bar;
mod wayland;

use tiny_skia::PixmapMut;

use crate::{
    ui::{Ui, Event},
    geometry::Size,
    wayland::{BarWindow, WaylandEvent}
};

fn main() {
    let mut window = BarWindow::new();
    let mut ui = Ui::new(bar::build);

    loop {
        for event in window.events_blocking() {
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
