use std::any::Any;

use mibar_core::{
    widget::{SizeConstraints, Element, Widget},
    Size, Rect, Context, DrawCtx, LayoutCtx,
    Id, StateHandle
};

use super::{SubscriptionToken, Event as TrayEvent, subscribe};

pub struct Tray;

#[derive(Default)]
pub struct TrayWidget;

pub struct State {
    _token: Option<SubscriptionToken>
}

impl Element for Tray {
    type Widget = TrayWidget;

    fn make_state(self, id: Id, ctx: &mut Context) -> <Self::Widget as Widget>::State {
        let token = subscribe(
            ctx.ui.runtime_handle(),
            ctx.ui.window_id(),
            ctx.ui.value_sender(id)
        );

        State { _token: Some(token) }
    }
}

impl Widget for TrayWidget {
    type State = State;

    fn layout(handle: StateHandle<Self::State>, _ctx: &mut LayoutCtx, bounds: SizeConstraints) -> Size {
        bounds.min
    }

    fn task_result(handle: StateHandle<Self::State>, _ctx: &mut Context, data: Box<dyn Any>) {
        let event = data.downcast::<TrayEvent>().unwrap();
    }

    fn draw(handle: StateHandle<Self::State>, _ctx: &mut DrawCtx, layout: Rect) {
        
    }
}
