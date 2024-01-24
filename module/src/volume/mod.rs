pub mod pulseaudio;

use std::any::Any;

use tokio::{
    time::{Duration, sleep},
    task::JoinHandle
};

use mibar_core::{
    widget::{
        text::{self, Text},
        Element, Widget, SizeConstraints
    },
    Size, Rect, MouseEvent, MouseButton,
    InitCtx, UpdateCtx, DrawCtx, LayoutCtx,
    Event, TypedId, ValueSender, StateHandle
};

pub type FormatFn = fn(pulseaudio::State) -> String;

pub struct PulseAudioVolume {
    format: FormatFn,
    style: Option<text::StyleFn>
}

pub struct PulseAudioVolumeWidget;

pub struct State {
    format: FormatFn,
    text: TypedId<Text>,
    handle: JoinHandle<()>
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
        let handle = ctx.task_with_sender(|sender: ValueSender<pulseaudio::State>| {
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
                            sender.send(state);
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
            text: ctx.new_child(text),
            handle
        };

        (PulseAudioVolumeWidget, state)
    }
}

impl Widget for PulseAudioVolumeWidget {
    type State = State;

    fn layout(
        handle: StateHandle<Self::State>,
        ctx: &mut LayoutCtx,
        bounds: SizeConstraints
    ) -> Size {
        ctx.layout(ctx.tree[handle].text, bounds)
    }

    fn event(
        _handle: StateHandle<Self::State>,
        ctx: &mut UpdateCtx,
        event: &Event
    ) {
        if matches!(event, Event::Mouse(MouseEvent::MousePress { pos, button })
            if *button == MouseButton::Left && ctx.layout().contains(*pos)
        ) {
            pulseaudio::dispatch(pulseaudio::Request::ToggleMute);
        }
    }

    fn task_result(
        handle: StateHandle<Self::State>,
        ctx: &mut UpdateCtx,
        data: Box<dyn Any>
    ) {
        let pa_state = *data.downcast::<pulseaudio::State>().unwrap();

        let state = &ctx.tree[handle];
        let text = (state.format)(pa_state);

        ctx.message(state.text, text::Message::SetText(text));
    }

    fn draw(handle: StateHandle<Self::State>, ctx: &mut DrawCtx, _layout: Rect) {
        ctx.draw(ctx.tree[handle].text);
    }

    fn destroy(state: Self::State) {
        if pulseaudio::subscriber_count() <= 1 {
            pulseaudio::dispatch(pulseaudio::Request::Terminate);
        }
        
        state.handle.abort();
    }
}
