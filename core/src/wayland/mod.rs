pub mod bar;
pub mod side_panel;
pub mod popup;
pub(crate) mod wayland_window;
pub(crate) mod layer_shell_window;

use bar::Bar;
use side_panel::SidePanel;
use popup::{Popup, PopupWindowConfig};
use layer_shell_window::LayerShellWindowConfig;

use crate::{Point, Vector};

#[derive(Debug)]
pub enum Window {
    Bar(Bar),
    SidePanel(SidePanel),
    Popup(Popup)
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum WindowDimensions {
    /// The window will exactly have the provided width and height.
    Fixed((u32, u32)),
    /// The window will be sized to its content. The provided width and height
    /// will serve as the maximum dimensions that the window is allowed to have.
    /// This constraint serves as a safety net in case the window ends up having a flex layout throughout.
    Auto((u32, u32))
}

#[derive(Clone, Copy, Debug)]
pub enum MouseEvent {
    EnterWindow,
    LeaveWindow,
    MousePress {
        pos: Point,
        button: MouseButton
    },
    MouseRelease {
        pos: Point,
        button: MouseButton
    },
    MouseMove(Point),
    Scroll(MouseScrollDelta)
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum MouseButton {
    Left,
    Right,
    Middle
}

// This type follows the winit implementation.

/// The difference in the mouse scroll wheel or touchpad state represented
/// in either lines/rows or pixels.
/// A positive Y value indicates that the content is being moved down.
/// A positive X value indicates that the content is being moved right.
#[derive(Clone, Copy, Debug)]
pub enum MouseScrollDelta {
    /// Amount in lines or rows to scroll in the horizontal and vertical directions.
    Line {
        x: f32,
        y: f32
    },
    /// Amount in pixels to scroll in the horizontal and vertical direction.
    Pixel {
        x: f32,
        y: f32
    }
}

#[derive(Debug)]
pub(crate) enum WindowEvent {
    ScaleFactor(f32),
    Resize((u32, u32)),
    Mouse(MouseEvent)
}

#[derive(Debug)]
pub(crate) enum WindowConfig {
    LayerShell(LayerShellWindowConfig),
    Popup(PopupWindowConfig)
}

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

impl MouseScrollDelta {
    /// Get the delta values, disregarding the units.
    /// Use this if you only need the direction.
    #[inline]
    pub fn values(&self) -> Vector {
        match self {
            MouseScrollDelta::Line { x, y } => Vector::new(*x, *y),
            MouseScrollDelta::Pixel { x, y } => Vector::new(*x, *y)
        }
    }
}

impl From<Bar> for Window {
    fn from(window: Bar) -> Self {
        Self::Bar(window)
    }
}

impl From<SidePanel> for Window {
    fn from(panel: SidePanel) -> Self {
        Self::SidePanel(panel)
    }
}

impl From<Popup> for Window {
    fn from(popup: Popup) -> Self {
        Self::Popup(popup)
    }
}
