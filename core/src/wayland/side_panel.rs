use smithay_client_toolkit::shell::wlr_layer::{Anchor, Layer};

use super::{layer_shell_window::LayerShellWindowConfig, WindowDimensions};

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Location {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight
}

#[derive(Clone, Copy, Debug)]
pub struct SidePanel {
    pub size: WindowDimensions,
    pub location: Location
}

impl SidePanel {
    #[inline]
    pub fn new(size: WindowDimensions, location: Location) -> Self {
        Self { size, location }
    }
}

impl From<SidePanel> for LayerShellWindowConfig {
    fn from(panel: SidePanel) -> Self {
        let anchor = match panel.location {
            Location::TopLeft => Anchor::TOP | Anchor::LEFT,
            Location::TopRight => Anchor::TOP | Anchor::RIGHT,
            Location::BottomLeft => Anchor::BOTTOM | Anchor::LEFT,
            Location::BottomRight => Anchor::BOTTOM | Anchor::RIGHT
        };

        Self {
            anchor,
            layer: Layer::Top,
            size: panel.size,
            exclusive_zone: None
        }
    }
}
