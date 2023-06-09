use crate::{
    geometry::Size,
    positioner::Positioner,
    ui::DrawCtx
};
use super::{
    size_constraints::SizeConstraints,
    Widget
};

#[derive(Default)]
pub struct Ram {

}

impl Widget for Ram {
    fn layout(&mut self, bounds: SizeConstraints) -> Size {
        bounds.constrain(Size { width: 100f32, height: 20f32 })
    }

    fn draw(&mut self, ctx: &mut DrawCtx, positioner: Positioner) {
        ctx.fill_rect(positioner.bounds, ctx.theme.cold3);
    }
}
