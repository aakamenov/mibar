use crate::{
    geometry::Size,
    positioner::Positioner,
    ui::{DrawCtx, LayoutCtx}
};
use super::{
    size_constraints::SizeConstraints,
    Widget
};

#[derive(Default)]
pub struct Cpu {

}

impl Widget for Cpu {
    fn layout(&mut self, ctx: &mut LayoutCtx, bounds: SizeConstraints) -> Size {
        bounds.constrain(Size { width: 100f32, height: 20f32 })
    }

    fn draw(&mut self, ctx: &mut DrawCtx, positioner: Positioner) {
        ctx.fill_rect(positioner.bounds, ctx.ui.theme.cold1);
    }
}
