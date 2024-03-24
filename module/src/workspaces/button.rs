use mibar_core::{
    widget::{SizeConstraints, Element, Widget},
    MouseEvent, MouseButton, Size, Rect, Circle, Color, Event, TypedId,
    DrawCtx, LayoutCtx, Context, TextInfo, StateHandle, Id, Task
};

use super::hyprland;

const RADIUS: f32 = 10f32;
const TEXT_SIZE: f32 = 12f32;

pub type StyleFn = fn() -> Style;

#[derive(Clone, Copy, Debug)]
pub struct ButtonId(pub TypedId<Button>);

pub struct Button {
    id: u8,
    style: StyleFn
}

#[derive(Default)]
pub struct ButtonWidget;

pub struct State {
    id: u8,
    text: String,
    is_active: bool,
    is_hovered: bool,
    text_dimensions: Size,
    status: WorkspaceStatus,
    style: StyleFn
}

#[derive(Clone, Copy, Debug)]
pub struct Style {
    pub active: Color,
    pub empty: Color,
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

    fn make_state(self, _id: Id, _ctx: &mut Context) -> <Self::Widget as Widget>::State {
        State {
            id: self.id,
            text: String::from("0"),
            is_active: false,
            is_hovered: false,
            text_dimensions: Size::ZERO,
            status: WorkspaceStatus::default(),
            style: self.style
        }
    }
}

impl Widget for ButtonWidget {
    type State = State;

    fn layout(
        handle: StateHandle<Self::State>,
        ctx: &mut LayoutCtx,
        bounds: SizeConstraints
    ) -> Size {
        let diameter = RADIUS * 2f32;
        let size = Size::new(diameter, diameter);

        let state = &mut ctx.tree[handle];
        let info = TextInfo::new(&state.text, TEXT_SIZE)
            .with_font(ctx.ui.theme().font);

        state.text_dimensions = ctx.renderer.measure_text(&info, size);

        bounds.constrain(size)
    }

    fn event(handle: StateHandle<Self::State>, ctx: &mut Context, event: &Event) {
        let (state, layout) = ctx.tree.state_and_layout_mut(handle);

        match event {
            Event::Mouse(event) => match event {
                MouseEvent::MouseMove(pos) => {
                    if layout.contains(*pos) {
                        if !state.is_hovered && !state.status.is_current {
                            ctx.ui.request_redraw();
                        }

                        state.is_hovered = true;
                    } else if state.is_hovered {
                        state.is_hovered = false;
                        state.is_active = false;

                        if !state.status.is_current {
                            ctx.ui.request_redraw();
                        }
                    }
                }
                MouseEvent::MousePress { pos, button }
                    if matches!(button, MouseButton::Left) =>
                {
                    if layout.contains(*pos) && !state.status.is_current {
                        state.is_active = true;
                        ctx.ui.request_redraw();
                    }
                }
                MouseEvent::MouseRelease { pos, button }
                    if matches!(button, MouseButton::Left) =>
                {
                    if state.is_active &&
                        layout.contains(*pos) &&
                        !state.status.is_current
                    {
                        let id = state.id;
                        let _ = ctx.ui.spawn(Task::void(hyprland::change_workspace(id)));
                    }

                    let state = &mut ctx.tree[handle];
                    if state.is_active && !state.status.is_current {
                        ctx.ui.request_redraw();
                    }

                    state.is_active = false;
                }
                _ => { }
            },
            _ => { }
        }
    }

    fn draw(handle: StateHandle<Self::State>, ctx: &mut DrawCtx, layout: Rect) {
        let state = &ctx.tree[handle];
        let style = (state.style)();
        let center = layout.center();
        let has_windows = state.status.num_windows > 0;

        let (fill_color, border_color) = if state.status.is_current {
            (style.active, style.active)
        }  else if has_windows {
            let fill = if state.is_active {
                style.active
            } else {
                Color::TRANSPARENT
            };

            (fill, style.active)
        } else {
            if state.is_active {
                (style.active, style.active)
            } else if state.is_hovered {
                (Color::TRANSPARENT, style.active)
            } else {
                (Color::TRANSPARENT, style.empty)
            }
        };

        let radius = if state.is_active {
            layout.width / 4f32
        } else {
            layout.width / 2f32
        };

        ctx.renderer.fill_circle(
            Circle::new(
                center,
                radius,
                fill_color
            ).with_border(
                2f32,
                border_color
            )
        );

        if (has_windows || state.status.is_current) && !state.is_active {
            let mut rect = Rect::from_size(state.text_dimensions);
            rect.x = center.x - (rect.width / 2f32);
            rect.y = center.y - (rect.height / 2f32);
    
            let info = TextInfo::new(&state.text, TEXT_SIZE)
                .with_font(ctx.ui.theme().font);

            ctx.renderer.fill_text(
                &info,
                rect,
                if state.status.is_current {
                    style.selected_text_color
                } else if state.is_hovered {
                    style.active
                } else {
                    style.text_color
                }
            );
        }
    }
}

impl ButtonId {
    pub fn set_status(self, ctx: &mut Context, new: WorkspaceStatus) {
        let state = &mut ctx.tree[self.0];

        if new.is_current != state.status.is_current {
            state.status.is_current = new.is_current;
            ctx.ui.request_redraw();
        }

        if new.num_windows != state.status.num_windows {
            state.status.num_windows = new.num_windows;
            state.text = new.num_windows.to_string();
            ctx.ui.request_layout();
        }
    }
}
