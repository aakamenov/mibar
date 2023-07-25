use crate::{
    geometry::Size,
    ui::{InitCtx, DrawCtx, LayoutCtx, TypedId}
};
use super::{
    size_constraints::SizeConstraints,
    text::Text,
    Element, Widget
};

pub struct DateTime;
pub struct DateTimeWidget;
pub struct State {
    text: TypedId<Text>
}

impl Element for DateTime {
    type Widget = DateTimeWidget;
    type Message = ();

    fn make_widget(self, ctx: &mut InitCtx) -> (
        Self::Widget,
        <Self::Widget as Widget>::State
    ) {
        let text = ctx.new_child(Text::new("21:45 25/07/2023"));
        (DateTimeWidget, State { text })
    }
}

impl Widget for DateTimeWidget {
    type State = State;

    fn layout(state: &mut Self::State, ctx: &mut LayoutCtx, bounds: SizeConstraints) -> Size {
        ctx.layout(&state.text, bounds)
    }

    fn draw(state: &mut Self::State, ctx: &mut DrawCtx) {
        ctx.draw(&state.text);
    }
}
