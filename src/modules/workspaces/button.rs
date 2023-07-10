use tiny_skia::Color;

use crate::{
    wayland::MouseEvent,
    geometry::Size,
    widget::{size_constraints::SizeConstraints, Element, Widget},
    ui::{
        InitCtx, DrawCtx, LayoutCtx,
        UpdateCtx, Event
    },
    renderer::Circle
};

const RADIUS: f32 = 8f32;
const HOVER_OUTLINE: f32 = 2f32;
const TOTAL_RADIUS: f32 = RADIUS + HOVER_OUTLINE;

pub struct Button;

pub struct ButtonWidget;

pub struct State {
    is_hovered: bool
}

impl Element for Button {
    type Widget = ButtonWidget;
    type Message = ();

    fn make_widget(self, _ctx: &mut InitCtx) -> (
        Self::Widget,
        <Self::Widget as Widget>::State
    ) {
        (ButtonWidget, State { is_hovered: false })
    }
}

impl Widget for ButtonWidget {
    type State = State;

    fn layout(
        _state: &mut Self::State,
        _ctx: &mut LayoutCtx,
        bounds: SizeConstraints
    ) -> Size {
        bounds.constrain(
            Size::new(TOTAL_RADIUS * 2f32, TOTAL_RADIUS * 2f32)
        )
    }

    fn event(state: &mut Self::State, ctx: &mut UpdateCtx, event: &Event) {
        match event {
            Event::Mouse(event) => match event {
                MouseEvent::MouseMove(pos) => {
                    if ctx.layout().contains(*pos) {
                        if !state.is_hovered {
                            ctx.request_redraw();
                        }

                        state.is_hovered = true;
                    } else if state.is_hovered {
                        state.is_hovered = false;
                        ctx.request_redraw();
                    }
                },
                _ => { }
            }
        }
    }

    fn draw(state: &mut Self::State, ctx: &mut DrawCtx) {
        let layout = ctx.layout();

        if state.is_hovered {
            ctx.renderer.fill_circle(
                Circle::new(
                    layout.center(),
                    layout.width / 2f32,
                    Color::TRANSPARENT
                ).with_border(HOVER_OUTLINE, ctx.ui.theme.muted)
            );
        }

        let layout = layout.shrink(HOVER_OUTLINE);
        ctx.renderer.fill_circle(
            Circle::new(
                layout.center(),
                layout.width / 2f32,
                Color::TRANSPARENT
            ).with_border(2f32, ctx.ui.theme.warm1)
        );
    }
}
