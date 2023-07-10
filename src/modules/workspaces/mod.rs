mod hyprland;
mod button;

use std::any::Any;

use hyprland::{Workspace, start_listener_loop};

use crate::{
    geometry::Size,
    widget::{size_constraints::SizeConstraints, Element, Widget},
    ui::{
        InitCtx, DrawCtx, LayoutCtx,
        UpdateCtx, Event, ValueSender,
        TypedId
    }
};

const WORKSPACE_COUNT: usize = 8;
const SPACING: f32 = 3f32;

pub struct Workspaces;
pub struct WorkspacesWidget;

pub struct State {
    buttons: Vec<TypedId<button::Button>>
}

impl State {
    pub fn new() -> Self {
        Self {
            buttons: Vec::new()
        }
    }
}

impl Element for Workspaces {
    type Widget = WorkspacesWidget;
    type Message = ();

    fn make_widget(self, ctx: &mut InitCtx) -> (
        Self::Widget,
        <Self::Widget as Widget>::State
    ) {
        ctx.task_with_sender(|sender: ValueSender<Vec<Workspace>>| {
            start_listener_loop(sender)
        });

        (WorkspacesWidget, State::new())
    }
}

impl Widget for WorkspacesWidget {
    type State = State;

    fn event(state: &mut Self::State, ctx: &mut UpdateCtx, event: &Event) {
        for button in &state.buttons {
            ctx.event(button, event);
        }
    }

    fn task_result(state: &mut Self::State, ctx: &mut UpdateCtx, data: Box<dyn Any>) {
        let workspaces = data.downcast::<Vec<Workspace>>().unwrap();

        for button in state.buttons.drain(..) {
            ctx.dealloc_child(button);
        }

        for _ in 0..workspaces.len() {
            let id = ctx.new_child(button::Button);
            state.buttons.push(id);
        }

        ctx.request_layout();
    }

    fn layout(state: &mut Self::State, ctx: &mut LayoutCtx, bounds: SizeConstraints) -> Size {
        let mut width = 0f32;
        let mut height = 0f32;
        let mut available = bounds.max.width;

        for button in &state.buttons {
            let bounds = SizeConstraints::new(
                Size::ZERO,
                Size::new(available, bounds.max.height)
            );

            let size = ctx.layout(button, bounds);
            ctx.position(button, |rect| rect.x += width);

            let total = size.width + SPACING;
            width += total;
            height = height.max(size.height);

            available -= total;
        }

        width = (width - SPACING).max(0f32);
        
        bounds.constrain(Size::new(width, height))
    }

    fn draw(state: &mut Self::State, ctx: &mut DrawCtx) {
        for button in &state.buttons {
            ctx.draw(button);
        }
    }
}
