use smithay_client_toolkit::shell::wlr_layer::{Anchor, Layer};

use super::layer_shell_window::LayerShellWindowConfig;

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
    fn from(value: Bar) -> Self {
        let anchor = match value.location {
            Location::Top => Anchor::TOP | Anchor::LEFT | Anchor::RIGHT,
            Location::Bottom => Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT,
            Location::Left => Anchor::LEFT | Anchor::TOP | Anchor::BOTTOM,
            Location::Right => Anchor::RIGHT | Anchor::TOP | Anchor::BOTTOM
        };

        let desired_size = match value.location {
            Location::Top | Location::Bottom => (0, value.size),
            Location::Left | Location::Right => (value.size, 0)
        };

        Self {
            anchor,
            layer: Layer::Top,
            desired_size,
            exclusive_zone: Some(value.size as i32)
        }
    }
}
