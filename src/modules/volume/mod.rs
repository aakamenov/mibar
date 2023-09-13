pub mod pulseaudio;

use tokio::time::{Duration, sleep};

use crate::{
    geometry::Size,
    widget::{
        size_constraints::SizeConstraints,
        text::{self, Text},
        Element, Widget
    },
    ui::{InitCtx, DrawCtx, LayoutCtx, TypedId, ValueSender}
};

pub type FormatFn = fn(pulseaudio::State) -> String;

pub struct PulseAudioVolume {
    format: FormatFn,
    style: Option<text::StyleFn>
}

pub struct PulseAudioVolumeWidget;

pub struct State {
    format: FormatFn,
    text: TypedId<Text>
}

impl PulseAudioVolume {
    #[inline]
    pub fn new(format: FormatFn) -> Self {
        Self {
            format,
            style: None
        }
    }

    #[inline]
    pub fn style(mut self, style: text::StyleFn) -> Self {
        self.style = Some(style);

        self
    }
}

impl Element for PulseAudioVolume {
    type Widget = PulseAudioVolumeWidget;
    type Message = ();

    fn make_widget(self, ctx: &mut InitCtx) -> (
        Self::Widget,
        <Self::Widget as Widget>::State
    ) {
        ctx.task_with_sender(|sender: ValueSender<pulseaudio::State>| {
            async move {
                pulseaudio::init();
                let mut rx = pulseaudio::subscribe();

                loop {
                    let Some(subscription) = rx.as_mut() else {
                        while rx.is_none() {
                            sleep(Duration::from_secs(2)).await;
                            pulseaudio::init();
                            rx = pulseaudio::subscribe();
                        }

                        continue;
                    };

                    match subscription.changed().await {
                        Ok(_) => {
                            let state = subscription.borrow().clone();
                            sender.send(state).await;
                        }
                        Err(_) => {
                            // Channel was closed, attempt to start the client again.
                            rx = None;
                        }
                    }
                }
            }
        });

        let text = (self.format)(pulseaudio::State::default());
        let text = match self.style {
            Some(style) => Text::new(text).style(style),
            None => Text::new(text),
        };

        let state = State {
            format: self.format,
            text: ctx.new_child(text)
        };

        (PulseAudioVolumeWidget, state)
    }
}

impl Widget for PulseAudioVolumeWidget {
    type State = State;

    fn layout(
        state: &mut Self::State,
        ctx: &mut LayoutCtx,
        bounds: SizeConstraints
    ) -> Size {
        ctx.layout(&state.text, bounds)
    }

    fn task_result(
        state: &mut Self::State,
        ctx: &mut crate::ui::UpdateCtx,
        data: Box<dyn std::any::Any>
    ) {
        let pa_state = *data.downcast::<pulseaudio::State>().unwrap();
        let text = (state.format)(pa_state);

        ctx.message(&state.text, text::Message::SetText(text));
    }

    fn draw(state: &mut Self::State, ctx: &mut DrawCtx) {
        ctx.draw(&state.text);
    }
}
