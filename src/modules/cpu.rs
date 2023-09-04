use crate::{
    geometry::Size,
    widget::{
        size_constraints::SizeConstraints,
        Element, Widget, text::{self, Text}
    },
    ui::{InitCtx, DrawCtx, LayoutCtx, ValueSender, TypedId},
    sys_info
};

use tokio::time::{Duration, interval};

const UPDATE_INTERVAL: Duration = Duration::from_millis(1000);

pub struct Cpu {
    style: Option<text::StyleFn>
}

pub struct CpuWidget;

pub struct State {
    text: TypedId<Text>
}

impl Cpu {
    #[inline]
    pub fn new() -> Self {
        Self { style: None }
    }

    #[inline]
    pub fn style(mut self, style: text::StyleFn) -> Self {
        self.style = Some(style);

        self
    }
}

#[inline]
fn format(value: f64) -> String {
    format!("\u{e266} {}%", value.round())
}

impl Element for Cpu {
    type Widget = CpuWidget;
    type Message = ();

    fn make_widget(self, ctx: &mut InitCtx) -> (
        Self::Widget,
        <Self::Widget as Widget>::State
    ) {
        ctx.task_with_sender(|sender: ValueSender<f64>| {
            async move {
                let mut interval = interval(UPDATE_INTERVAL);

                loop {
                    interval.tick().await;
                    sender.send(sys_info::cpu_usage()).await;
                }
            }
        });

        let text = format(0f64);
        let text = match self.style {
            Some(style) => Text::new(text).style(style),
            None => Text::new(text),
        };
        
        let state = State {
            text: ctx.new_child(text)
        };

        (CpuWidget, state)
    }
}

impl Widget for CpuWidget {
    type State = State;

    fn layout(state: &mut Self::State, ctx: &mut LayoutCtx, bounds: SizeConstraints) -> Size {
        ctx.layout(&state.text, bounds)
    }

    fn task_result(
        state: &mut Self::State,
        ctx: &mut crate::ui::UpdateCtx,
        data: Box<dyn std::any::Any>
    ) {
        let usage = *data.downcast::<f64>().unwrap();
        let text = format(usage);

        ctx.message(&state.text, text::Message::SetText(text));
    }

    fn draw(state: &mut Self::State, ctx: &mut DrawCtx) {
        ctx.draw(&state.text);
    }
}
