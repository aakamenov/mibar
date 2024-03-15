pub mod widget;
pub mod reactive;
pub mod image;
mod widget_tree;
mod ui;
mod event_queue;
mod task;
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
pub use widget_tree::*;
pub use event_queue::*;
pub use task::Task;
pub use reactive::*;
pub use renderer::Renderer;
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
    pub use super::wayland::{Window, WindowDimensions};
    pub use super::wayland::bar;
    pub use super::wayland::side_panel;
    pub use super::wayland::popup;
}
