use crate::{
    wayland::{MouseEvent, MouseButton},
    geometry::{Size, Rect},
    widget::{size_constraints::SizeConstraints, Element, Widget},
    ui::{
        InitCtx, DrawCtx, LayoutCtx,
        UpdateCtx, Event
    },
    draw::Circle,
    renderer::TextInfo,
    color::Color
};

use super::hyprland;

const RADIUS: f32 = 10f32;
const TEXT_SIZE: f32 = 12f32;

pub type StyleFn = fn() -> Style;

pub struct Button {
    id: u8,
    style: StyleFn
}

pub struct ButtonWidget;

pub struct State {
    id: u8,
    is_active: bool,
    is_hovered: bool,
    text_info: TextInfo,
    text_size: Size,
    status: WorkspaceStatus,
    style: StyleFn
}

#[derive(Clone, Copy, Debug)]
pub struct Style {
    pub active: ButtonStyle,
    pub inactive: ButtonStyle,
    pub text_color: Color,
    pub selected_text_color: Color 
}

#[derive(Clone, Copy, Debug)]
pub struct ButtonStyle {
    pub color: Color,
    pub hovered: Color,
    pub active: Color
}

#[derive(Clone, Copy, PartialEq, Default, Debug)]
pub struct WorkspaceStatus {
    pub is_current: bool,
    pub num_windows: u8
}

impl Button {
    #[inline]
    pub fn new(id: u8, style: StyleFn) -> Self {
        Self { id, style }
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
            is_active: false,
            is_hovered: false,
            text_info: TextInfo::new("0", TEXT_SIZE)
                .with_font(ctx.theme().font),
            text_size: Size::ZERO,
            status: WorkspaceStatus::default(),
            style: self.style
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
                        state.is_active = true;
                        ctx.request_redraw();
                    }
                }
                MouseEvent::MouseRelease { pos, button }
                    if matches!(button, MouseButton::Left) =>
                {
                    if state.is_active &&
                        ctx.layout().contains(*pos) &&
                        !state.status.is_current
                    {
                        let _ = ctx.task_void(hyprland::change_workspace(state.id));
                    }

                    if state.is_active && !state.status.is_current {
                        ctx.request_redraw();
                    }

                    state.is_active = false;
                }
                _ => { }
            }
        }
    }

    fn draw(state: &mut Self::State, ctx: &mut DrawCtx) {
        let style = (state.style)();
        let layout = ctx.layout();
        let center = layout.center();
        let has_windows = state.status.num_windows > 0;

        let button_style = if has_windows || state.status.is_current {
            style.active
        } else {
            style.inactive
        };

        let fill = if state.status.is_current {
            button_style.color
        } else if state.is_active {
            button_style.active
        } else if state.is_hovered {
            button_style.hovered
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
                button_style.color
            )
        );

        if has_windows || state.status.is_current {
            let mut rect = Rect::from_size(state.text_size);
            rect.x = center.x - (rect.width / 2f32);
            rect.y = center.y - (rect.height / 2f32);
    
            ctx.renderer.fill_text(
                &state.text_info,
                rect,
                if state.status.is_current ||
                    state.is_hovered ||
                    state.is_active
                {
                    style.selected_text_color
                } else {
                    style.text_color
                }
            );
        }
    }
}
