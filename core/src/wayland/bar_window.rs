use smithay_client_toolkit::shell::wlr_layer::{LayerSurface, Anchor, Layer};

use super::layer_shell_window::LayerShellWindow;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Location {
    Top,
    Bottom,
    Left,
    Right
}

pub struct BarWindow {
    pub location: Location,
    pub size: u32
}

impl LayerShellWindow for BarWindow {
    fn surface_layer(&self) -> Layer {
        Layer::Top
    }

    fn desired_size(&self) -> (u32, u32) {
        unreachable!("An anchored surface should always have a size.") // I think...
    }

    fn configure_surface(&self, surface: &LayerSurface) {
        let anchor = match self.location {
            Location::Top => Anchor::TOP | Anchor::LEFT | Anchor::RIGHT,
            Location::Bottom => Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT,
            Location::Left => Anchor::LEFT | Anchor::TOP | Anchor::BOTTOM,
            Location::Right => Anchor::RIGHT | Anchor::TOP | Anchor::BOTTOM
        };

        surface.set_anchor(anchor);

        match self.location {
            Location::Top | Location::Bottom => surface.set_size(0, self.size),
            Location::Left | Location::Right => surface.set_size(self.size, 0)
        };

        surface.set_exclusive_zone(self.size as i32);
    }
}
