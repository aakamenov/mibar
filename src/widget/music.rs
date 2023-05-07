use crate::{
    geometry::Size,
    ui::{DrawCtx, LayoutCtx}
};
use super::{
    size_constraints::SizeConstraints,
    Widget
};

#[derive(Default)]
pub struct Music {

}

impl Widget for Music {
    fn layout(&mut self, ctx: &mut LayoutCtx, bounds: SizeConstraints) -> Size {
        bounds.constrain(Size { width: bounds.max.width, height: 20f32 })
    }

    fn draw(&mut self, ctx: &mut DrawCtx) {
        ctx.fill_rect(ctx.layout(), ctx.ui.theme.warm2);
    }
}
