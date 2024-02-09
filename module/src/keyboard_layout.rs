use std::any::Any;

use mibar_core::{
    widget::{
        button::{self, Button}, text::Text,
        SizeConstraints, Element, Widget
    },
    Size, Rect, DrawCtx, LayoutCtx, Id, Task,
    Context, Event, TypedId, StateHandle
};

use crate::hyprland::{
    self,
    KeyboardSubscriber,
    KeyboardLayoutChanged,
    SubscriptionToken
};

pub struct KeyboardLayout {
    device: &'static str,
    button: Button<Text>
}

#[derive(Default)]
pub struct KeyboardLayoutWidget;

pub struct State {
    button: TypedId<Button<Text>>,
    _token: SubscriptionToken<KeyboardSubscriber>
}

impl KeyboardLayout {
    #[inline]
    pub fn new(device: &'static str) -> Self {
        Self {
            device,
            button: Button::new("n/a", move |ctx| {
                let _ = ctx.ui.spawn(Task::void(hyprland::keyboard_layout_next(device)));
            })
        }
    }

    #[inline]
    pub fn style(mut self, style: button::StyleFn) -> Self {
        self.button = self.button.style(style);

        self
    }
}

impl Element for KeyboardLayout {
    type Widget = KeyboardLayoutWidget;

    fn make_state(self, id: Id, ctx: &mut Context) -> <Self::Widget as Widget>::State {
        let _ = ctx.ui.spawn(Task::new(id, async move {
            let layout = hyprland::current_layout(self.device).await;

            KeyboardLayoutChanged {
                layout,
                device: self.device
            }
        }));

        let token = hyprland::subscribe_keyboard_layout(
            ctx.ui.runtime_handle(),
            ctx.ui.window_id(),
            ctx.ui.value_sender(id),
            self.device
        );

        State {
            button: ctx.new_child(id, self.button),
            _token: token
        }
    }
}

impl Widget for KeyboardLayoutWidget {
    type State = State;

    fn layout(
        handle: StateHandle<Self::State>,
        ctx: &mut LayoutCtx,
        bounds: SizeConstraints
    ) -> Size {
        ctx.layout(ctx.tree[handle].button, bounds)
    }

    fn event(handle: StateHandle<Self::State>, ctx: &mut Context, event: &Event) {
        ctx.event(ctx.tree[handle].button, event);
    }

    fn task_result(handle: StateHandle<Self::State>, ctx: &mut Context, data: Box<dyn Any>) {
        let event = data.downcast::<KeyboardLayoutChanged>().unwrap();
        let button = ctx.tree[handle].button;
        let text = button.child(ctx);

        match event.layout {
            Some(layout) => text.set_text(ctx, layout),
            None => text.set_text(ctx, "n/a")
        }
    }

    fn draw(handle: StateHandle<Self::State>, ctx: &mut DrawCtx, _layout: Rect) {
        ctx.draw(ctx.tree[handle].button);
    }
}
