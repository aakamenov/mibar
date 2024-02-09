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
    Context, DrawCtx, LayoutCtx, Id, Task,
    Event, TypedId, ValueSender, StateHandle
};

pub type FormatFn = fn(pulseaudio::State) -> String;

pub struct PulseAudioVolume {
    format: FormatFn,
    style: Option<text::StyleFn>
}

#[derive(Default)]
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

    fn make_state(self, id: Id, ctx: &mut Context) -> <Self::Widget as Widget>::State {
        let task = Task::with_sender(id, |sender: ValueSender<pulseaudio::State>| {
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
        let handle = ctx.ui.spawn(task);

        let text = (self.format)(pulseaudio::State::default());
        let text = match self.style {
            Some(style) => Text::new(text).style(style),
            None => Text::new(text),
        };

        State {
            format: self.format,
            text: ctx.new_child(id, text),
            handle
        }
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
        handle: StateHandle<Self::State>,
        ctx: &mut Context,
        event: &Event
    ) {
        let layout = ctx.tree.layout(handle);

        if matches!(event, Event::Mouse(MouseEvent::MousePress { pos, button })
            if *button == MouseButton::Left && layout.contains(*pos)
        ) {
            pulseaudio::dispatch(pulseaudio::Request::ToggleMute);
        }
    }

    fn task_result(
        handle: StateHandle<Self::State>,
        ctx: &mut Context,
        data: Box<dyn Any>
    ) {
        let pa_state = *data.downcast::<pulseaudio::State>().unwrap();

        let state = &ctx.tree[handle];
        let text = (state.format)(pa_state);

        let child = state.text;
        child.set_text(ctx, text);
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
