use crate::{
    geometry::Size,
    ui::{InitCtx, DrawCtx, LayoutCtx},
    renderer::Quad
};
use super::{
    size_constraints::SizeConstraints,
    Element, Widget
};

pub struct Cpu;
pub struct CpuWidget;
pub struct State;

impl Element for Cpu {
    type Widget = CpuWidget;
    type Message = ();

    fn make_widget(self, _ctx: &mut InitCtx) -> (
        Self::Widget,
        <Self::Widget as Widget>::State
    ) {
        (CpuWidget, State)
    }
}

impl Widget for CpuWidget {
    type State = State;

    fn layout(_state: &mut Self::State, _ctx: &mut LayoutCtx, bounds: SizeConstraints) -> Size {
        bounds.constrain(Size { width: 100f32, height: 20f32 })
    }

    fn draw(_state: &mut Self::State, ctx: &mut DrawCtx) {
        ctx.renderer.fill_quad(
            Quad::rounded(ctx.layout(), ctx.theme().cold1, 6f32)
        );
    }
}
