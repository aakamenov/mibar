use crate::{
    geometry::Size,
    ui::{DrawCtx, LayoutCtx}
};
use super::{
    size_constraints::SizeConstraints,
    Widget
};

#[derive(Default)]
pub struct Ram {

}

impl Widget for Ram {
    fn layout(&mut self, ctx: &mut LayoutCtx, bounds: SizeConstraints) -> Size {
        bounds.constrain(Size { width: 100f32, height: 20f32 })
    }

    fn draw(&mut self, ctx: &mut DrawCtx) {
        ctx.fill_rect(ctx.layout(), ctx.ui.theme.cold3);
    }
}
