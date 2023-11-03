use smithay_client_toolkit::shell::wlr_layer::{Anchor, Layer};

use super::layer_shell_window::LayerShellWindowConfig;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Location {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight
}

#[derive(Clone, Copy, Debug)]
pub struct SidePanel {
    pub size: (u32, u32),
    pub location: Location
}

impl SidePanel {
    #[inline]
    pub fn new(size: (u32, u32), location: Location) -> Self {
        Self { size, location }
    }
}

impl From<SidePanel> for LayerShellWindowConfig {
    fn from(value: SidePanel) -> Self {
        let anchor = match value.location {
            Location::TopLeft => Anchor::TOP | Anchor::LEFT,
            Location::TopRight => Anchor::TOP | Anchor::RIGHT,
            Location::BottomLeft => Anchor::BOTTOM | Anchor::LEFT,
            Location::BottomRight => Anchor::BOTTOM | Anchor::RIGHT
        };

        Self {
            anchor,
            layer: Layer::Top,
            desired_size: value.size,
            exclusive_zone: None
        }
    }
}
