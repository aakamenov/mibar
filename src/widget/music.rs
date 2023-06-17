use crate::{
    geometry::Size,
    ui::{InitCtx, DrawCtx, LayoutCtx}
};
use super::{
    size_constraints::SizeConstraints,
    Element, Widget
};

pub struct Music;
pub struct MusicWidget;
pub struct State;

impl Element for Music {
    type Widget = MusicWidget;
    type Message = ();

    fn make_widget(self, _ctx: &mut InitCtx) -> (
        Self::Widget,
        <Self::Widget as Widget>::State
    ) {
        (MusicWidget, State)
    }
}

impl Widget for MusicWidget {
    type State = State;

    fn layout(_state: &mut Self::State, _ctx: &mut LayoutCtx, bounds: SizeConstraints) -> Size {
        bounds.constrain(Size { width: bounds.max.width, height: 20f32 })
    }

    fn draw(_state: &mut Self::State, ctx: &mut DrawCtx) {
        ctx.fill_rect(ctx.layout(), ctx.ui.theme.warm2);
    }
}
