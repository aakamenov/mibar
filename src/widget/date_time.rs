use crate::{
    geometry::Size,
    ui::{InitCtx, DrawCtx, LayoutCtx}
};
use super::{
    size_constraints::SizeConstraints,
    Element, Widget
};

pub struct DateTime;
pub struct DateTimeWidget;
pub struct State;

impl Element for DateTime {
    type Widget = DateTimeWidget;
    type Message = ();

    fn make_widget(self, _ctx: &mut InitCtx) -> (
        Self::Widget,
        <Self::Widget as Widget>::State
    ) {
        (DateTimeWidget, State)
    }
}

impl Widget for DateTimeWidget {
    type State = State;

    fn layout(_state: &mut Self::State, _ctx: &mut LayoutCtx, bounds: SizeConstraints) -> Size {
        bounds.constrain(Size { width: 100f32, height: 20f32 })
    }

    fn draw(_state: &mut Self::State, ctx: &mut DrawCtx) {
        ctx.fill_rect(ctx.layout(), ctx.ui.theme.warm1);
    }
}
