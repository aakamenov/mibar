use smithay_client_toolkit::shell::wlr_layer::{Anchor, Layer};

use super::{layer_shell_window::LayerShellWindowConfig, WindowDimensions};

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Location {
    Top,
    Bottom,
    Left,
    Right
}

#[derive(Clone, Copy, Debug)]
pub struct Bar {
    pub location: Location,
    pub size: u32
}

impl Bar {
    #[inline]
    pub fn new(size: u32, location: Location) -> Self {
        Self { size, location }
    }
}

impl From<Bar> for LayerShellWindowConfig {
    fn from(bar: Bar) -> Self {
        let anchor = match bar.location {
            Location::Top => Anchor::TOP | Anchor::LEFT | Anchor::RIGHT,
            Location::Bottom => Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT,
            Location::Left => Anchor::LEFT | Anchor::TOP | Anchor::BOTTOM,
            Location::Right => Anchor::RIGHT | Anchor::TOP | Anchor::BOTTOM
        };

        let size = match bar.location {
            Location::Top | Location::Bottom => (0, bar.size),
            Location::Left | Location::Right => (bar.size, 0)
        };

        Self {
            anchor,
            layer: Layer::Top,
            size: WindowDimensions::Fixed(size),
            exclusive_zone: Some(bar.size as i32)
        }
    }
}
