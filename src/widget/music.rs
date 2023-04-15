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
pub struct Music {

}

impl Widget for Music {
    fn layout(&mut self, bounds: SizeConstraints) -> Size {
        bounds.constrain(Size { width: bounds.max.width, height: 20f32 })
    }

    fn draw(&mut self, ctx: &mut DrawCtx, positioner: Positioner) {
        ctx.fill_rect(positioner.bounds, ctx.theme.warm2);
    }
}
