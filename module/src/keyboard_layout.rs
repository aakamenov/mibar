use std::any::Any;

use mibar_core::{
    widget::{
        button::{self, Button}, text::{self, Text},
        SizeConstraints, Element, Widget
    },
    Size, Rect, InitCtx, DrawCtx, LayoutCtx,
    UpdateCtx, Event, TypedId, StateHandle
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
                let _ = ctx.task_void(hyprland::keyboard_layout_next(device));
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
    type Message = ();

    fn make_widget(self, ctx: &mut InitCtx) -> (
        Self::Widget,
        <Self::Widget as Widget>::State
    ) {
        let _ = ctx.task(async move {
            let layout = hyprland::current_layout(self.device).await;

            KeyboardLayoutChanged {
                layout,
                device: self.device
            }
        });

        let token = hyprland::subscribe_keyboard_layout(
            ctx.ui.runtime_handle(),
            ctx.ui.window_id(),
            ctx.value_sender(),
            self.device
        );

        (
            KeyboardLayoutWidget,
            State {
                button: ctx.new_child(self.button),
                _token: token
            }
        )
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

    fn event(handle: StateHandle<Self::State>, ctx: &mut UpdateCtx, event: &Event) {
        ctx.event(ctx.tree[handle].button, event);
    }

    fn task_result(handle: StateHandle<Self::State>, ctx: &mut UpdateCtx, data: Box<dyn Any>) {
        let event = data.downcast::<KeyboardLayoutChanged>().unwrap();
        let button = ctx.tree[handle].button;

        match event.layout {
            Some(layout) =>
                ctx.message(button, text::Message::SetText(layout.into())),
            None =>
                ctx.message(button, text::Message::SetText("n/a".into()))
        };
    }

    fn draw(handle: StateHandle<Self::State>, ctx: &mut DrawCtx, _layout: Rect) {
        ctx.draw(ctx.tree[handle].button);
    }
}
