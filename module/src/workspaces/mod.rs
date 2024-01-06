mod button;

pub use button::{StyleFn, Style, ButtonStyle};

use std::{any::Any, mem::MaybeUninit};

use mibar_core::{
    widget::{SizeConstraints, Element, Widget},
    MouseEvent, Size, InitCtx, DrawCtx, LayoutCtx,
    UpdateCtx, Event, ValueSender, TypedId
};

use crate::hyprland::{self, WorkspacesChanged, SubscriptionToken};

const WORKSPACE_COUNT: usize = 8;
const SPACING: f32 = 4f32;

pub struct Workspaces {
    style: StyleFn
}
pub struct WorkspacesWidget;

pub struct State {
    buttons: [TypedId<button::Button>; WORKSPACE_COUNT],
    _token: SubscriptionToken<ValueSender<WorkspacesChanged>>
}

impl Workspaces {
    #[inline]
    pub fn new(style: StyleFn) -> Self {
        Self { style }
    }
}

impl Element for Workspaces {
    type Widget = WorkspacesWidget;
    type Message = ();

    fn make_widget(self, ctx: &mut InitCtx) -> (
        Self::Widget,
        <Self::Widget as Widget>::State
    ) {
        let token = hyprland::subscribe_workspaces(
            ctx.runtime_handle(),
            ctx.window_id(),
            ctx.value_sender()
        );

        let mut buttons: MaybeUninit<[TypedId<button::Button>; WORKSPACE_COUNT]> =
            MaybeUninit::uninit();

        for i in 0..WORKSPACE_COUNT {
            let button = ctx.new_child(
                button::Button::new((i + 1) as u8, self.style)
            );
            unsafe {
                buttons.assume_init_mut()[i] = button;
            }
        }

        let buttons = unsafe { buttons.assume_init() };

        (WorkspacesWidget, State { buttons, _token: token })
    }
}

impl Widget for WorkspacesWidget {
    type State = State;

    fn event(state: &mut Self::State, ctx: &mut UpdateCtx, event: &Event) {
        if let Event::Mouse(MouseEvent::Scroll(delta)) = event {
            if ctx.is_hovered() {
                let y = delta.values().y;
                if y > 0f32 {
                    let _ = ctx.task_void(hyprland::move_workspace_next());
                } else if y < 0f32 {
                    let _ = ctx.task_void(hyprland::move_workspace_prev());
                }

                return;
            }
        }

        for button in &state.buttons {
            ctx.event(button, event);
        }
    }

    fn task_result(
        state: &mut Self::State,
        ctx: &mut UpdateCtx,
        data: Box<dyn Any>
    ) {
        let event = data.downcast::<WorkspacesChanged>().unwrap();
        let mut empty = [true; WORKSPACE_COUNT];

        for workspace in event.workspaces {
            let index = (workspace.id - 1) as usize;
            if index >= WORKSPACE_COUNT {
                continue;
            }

            empty[index] = false;

            let msg = button::WorkspaceStatus {
                is_current: workspace.id == event.current,
                num_windows: workspace.num_windows
            };
            ctx.message(&state.buttons[index], msg);
        }

        for (i, is_empty) in empty.into_iter().enumerate() {
            if !is_empty {
                continue;
            }

            let msg = button::WorkspaceStatus {
                is_current: i + 1 == event.current as usize,
                num_windows: 0
            };
            ctx.message(&state.buttons[i], msg);
        }
    }

    fn layout(
        state: &mut Self::State,
        ctx: &mut LayoutCtx,
        bounds: SizeConstraints
    ) -> Size {
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
