pub mod widget;
mod ui;
mod geometry;
mod theme;
mod renderer;
mod wayland;
mod color;
mod draw;
mod gradient;
mod client;

pub use client::run;
pub use ui::*;
pub use geometry::*;
pub use theme::*;
pub use wayland::{MouseEvent, MouseButton, MouseScrollDelta};
pub use color::*;
pub use draw::*;
pub use gradient::*;
pub use tokio;
pub use cosmic_text::{Family, Stretch, Style, Weight};

pub mod window {
    pub use super::client::WindowId;
    pub use super::wayland::Window;
    pub use super::wayland::bar;
    pub use super::wayland::side_panel;
    pub use super::wayland::popup;
}
