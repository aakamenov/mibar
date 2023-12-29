use std::any::Any;

use mibar_core::{
    widget::{
        button::{self, Button}, text::{self, Text},
        SizeConstraints, Element, Widget
    },
    Size, InitCtx, DrawCtx, LayoutCtx,
    UpdateCtx, Event, TypedId
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
            button: Button::new("n/a", move |ctx, _| {
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
            ctx.runtime_handle(),
            ctx.window_id(),
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
        state: &mut Self::State,
        ctx: &mut LayoutCtx,
        bounds: SizeConstraints
    ) -> Size {
        ctx.layout(&state.button, bounds)
    }

    fn event(state: &mut Self::State, ctx: &mut UpdateCtx, event: &Event) {
        ctx.event(&state.button, event);
    }

    fn task_result(state: &mut Self::State, ctx: &mut UpdateCtx, data: Box<dyn Any>) {
        let event = data.downcast::<KeyboardLayoutChanged>().unwrap();

        match event.layout {
            Some(layout) =>
                ctx.message(&state.button, text::Message::SetText(layout.into())),
            None =>
                ctx.message(&state.button, text::Message::SetText("n/a".into()))
        };
    }

    fn draw(state: &mut Self::State, ctx: &mut DrawCtx) {
        ctx.draw(&state.button);
    }
}
