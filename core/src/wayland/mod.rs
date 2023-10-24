pub(crate) mod layer_shell_window;
pub(crate) mod bar_window;
pub (crate) mod panel;

use bar_window::BarWindow;
use panel::Panel;

use crate::{Point, Vector};

pub enum Window {
    Bar(BarWindow),
    Panel(Panel)
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

impl From<BarWindow> for Window {
    fn from(window: BarWindow) -> Self {
        Self::Bar(window)
    }
}

impl From<Panel> for Window {
    fn from(panel: Panel) -> Self {
        Self::Panel(panel)
    }
}
