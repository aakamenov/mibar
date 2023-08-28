use std::any::Any;

use tiny_skia::Color;

use crate::{
    wayland::{MouseEvent, MouseButton},
    geometry::{Size, Rect},
    widget::{size_constraints::SizeConstraints, Element, Widget},
    ui::{
        InitCtx, DrawCtx, LayoutCtx,
        UpdateCtx, Event
    },
    renderer::{Circle, TextInfo}
};
use super::hyprland;

const RADIUS: f32 = 10f32;
const TEXT_SIZE: f32 = 12f32;
const HOVER_OPACITY: f32 = 0.7;
const FOCUS_OPACITY: f32 = 0.5;

pub struct Button {
    id: u8
}

pub struct ButtonWidget;

pub struct State {
    id: u8,
    is_focused: bool,
    is_hovered: bool,
    text_info: TextInfo,
    text_size: Size,
    status: WorkspaceStatus
}

#[derive(Clone, Copy, PartialEq, Default, Debug)]
pub struct WorkspaceStatus {
    pub is_current: bool,
    pub num_windows: u8
}

impl Button {
    #[inline]
    pub fn new(id: u8) -> Self {
        Self { id }
    }
}

impl Element for Button {
    type Widget = ButtonWidget;
    type Message = WorkspaceStatus;

    fn make_widget(self, ctx: &mut InitCtx) -> (
        Self::Widget,
        <Self::Widget as Widget>::State
    ) {
        let state = State {
            id: self.id,
            is_focused: false,
            is_hovered: false,
            text_info: TextInfo::new("0", TEXT_SIZE)
                .with_font(ctx.theme().font),
            text_size: Size::ZERO,
            status: WorkspaceStatus::default()
        };

        (ButtonWidget, state)
    }

    fn message(
        state: &mut <Self::Widget as Widget>::State,
        ctx: &mut UpdateCtx,
        msg: Self::Message
    ) {
        if msg.is_current != state.status.is_current {
            state.status.is_current = msg.is_current;
            ctx.request_redraw();
        }

        if msg.num_windows != state.status.num_windows {
            state.status.num_windows = msg.num_windows;
            state.text_info.text = msg.num_windows.to_string();
            ctx.request_layout();
        }
    }
}

impl Widget for ButtonWidget {
    type State = State;

    fn layout(
        state: &mut Self::State,
        ctx: &mut LayoutCtx,
        bounds: SizeConstraints
    ) -> Size {
        let diameter = RADIUS * 2f32;
        let size = Size::new(diameter, diameter);

        state.text_size = ctx.measure_text(&state.text_info, size);

        bounds.constrain(size)
    }

    // Task returns no value.
    fn task_result(_state: &mut Self::State, _ctx: &mut UpdateCtx, _data: Box<dyn Any>) { }

    fn event(state: &mut Self::State, ctx: &mut UpdateCtx, event: &Event) {
        match event {
            Event::Mouse(event) => match event {
                MouseEvent::MouseMove(pos) => {
                    if ctx.layout().contains(*pos) {
                        if !state.is_hovered && !state.status.is_current {
                            ctx.request_redraw();
                        }

                        state.is_hovered = true;
                    } else if state.is_hovered {
                        state.is_hovered = false;

                        if !state.status.is_current {
                            ctx.request_redraw();
                        }
                    }
                }
                MouseEvent::MousePress { pos, button }
                    if matches!(button, MouseButton::Left) =>
                {
                    if ctx.layout().contains(*pos) && !state.status.is_current {
                        state.is_focused = true;
                        ctx.request_redraw();
                    }
                }
                MouseEvent::MouseRelease { pos, button }
                    if matches!(button, MouseButton::Left) =>
                {
                    if state.is_focused &&
                        ctx.layout().contains(*pos) &&
                        !state.status.is_current
                    {
                        ctx.task(hyprland::change_workspace(state.id));
                    }

                    if state.is_focused && !state.status.is_current {
                        ctx.request_redraw();
                    }

                    state.is_focused = false;
                }
                _ => { }
            }
        }
    }

    fn draw(state: &mut Self::State, ctx: &mut DrawCtx) {
        let layout = ctx.layout();
        let center = layout.center();
        let has_windows = state.status.num_windows > 0;

        let color = if has_windows || state.status.is_current {
            ctx.theme().warm1
        } else {
            ctx.theme().muted
        };

        let fill = if state.status.is_current {
            color
        } else if state.is_focused {
            let mut color = color.clone();
            color.apply_opacity(FOCUS_OPACITY);

            color
        } else if state.is_hovered {
            let mut color = color.clone();
            color.apply_opacity(HOVER_OPACITY);

            color
        } else {
            Color::TRANSPARENT
        };

        ctx.renderer.fill_circle(
            Circle::new(
                center,
                layout.width / 2f32,
                fill
            ).with_border(
                2f32,
                color
            )
        );

        if has_windows || state.status.is_current {
            let mut rect = Rect::from_size(state.text_size);
            rect.x = center.x - (rect.width / 2f32);
            rect.y = center.y - (rect.height / 2f32);
    
            ctx.renderer.fill_text(
                &state.text_info,
                rect,
                if state.status.is_current {
                    ctx.theme().base
                } else {
                    ctx.theme().text
                }
            );
        }
    }
}
