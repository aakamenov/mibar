use smithay_client_toolkit::shell::wlr_layer::{LayerSurface, Anchor, Layer};

use super::layer_shell_window::LayerShellWindow;

pub struct Panel {
    pub size: (u32, u32)
}

impl Panel {
    #[inline]
    pub fn new(size: (u32, u32)) -> Self {
        Self { size }
    }
}

impl LayerShellWindow for Panel {
    fn surface_layer(&self) -> Layer {
        Layer::Top
    }

    fn desired_size(&self) -> (u32, u32) {
        self.size
    }

    fn configure_surface(&self, surface: &LayerSurface) {
        surface.set_anchor(Anchor::TOP | Anchor::RIGHT);
        surface.set_margin(20, 20, 0, 0);
        surface.set_size(self.size.0, self.size.1);
    }
}
