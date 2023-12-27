use mibar_core::{
    widget::{
        text::{self, Text},
        Element, Widget, SizeConstraints
    },
    InitCtx, DrawCtx, LayoutCtx, UpdateCtx,
    ValueSender, TypedId, Size
};

use tokio::task::JoinHandle;

use crate::sys_info::{self, RamUsage};

pub struct Ram {
    style: Option<text::StyleFn>
}

pub struct RamWidget;

pub struct State {
    text: TypedId<Text>,
    handle: JoinHandle<()>
}

impl Ram {
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
fn format(ram: RamUsage) -> String {
    let used = ram.used() as f64 / 1024f64 / 1024f64;
    let total = ram.total as f64 / 1024f64 / 1024f64;

    format!("\u{f4bc} {:.1}/{:.1} GB", used, total)
}

impl Element for Ram {
    type Widget = RamWidget;
    type Message = ();

    fn make_widget(self, ctx: &mut InitCtx) -> (
        Self::Widget,
        <Self::Widget as Widget>::State
    ) {
        let mut rx = sys_info::RAM.subscribe(ctx.runtime_handle());
        let handle = ctx.task_with_sender(|sender: ValueSender<RamUsage>| {
            async move {
                loop {
                    if let Ok(value) = rx.recv().await {
                        sender.send(*value);
                    }
                }
            }
        });

        let text = format(RamUsage::default());
        let text = match self.style {
            Some(style) => Text::new(text).style(style),
            None => Text::new(text),
        };

        let state = State {
            text: ctx.new_child(text),
            handle
        };

        (RamWidget, state)
    }
}

impl Widget for RamWidget {
    type State = State;

    fn layout(state: &mut Self::State, ctx: &mut LayoutCtx, bounds: SizeConstraints) -> Size {
        ctx.layout(&state.text, bounds)
    }

    fn task_result(
        state: &mut Self::State,
        ctx: &mut UpdateCtx,
        data: Box<dyn std::any::Any>
    ) {
        let usage = *data.downcast::<RamUsage>().unwrap();
        let text = format(usage);

        ctx.message(&state.text, text::Message::SetText(text));
    }

    fn draw(state: &mut Self::State, ctx: &mut DrawCtx) {
        ctx.draw(&state.text);
    }

    fn destroy(state: Self::State) {
        state.handle.abort();
    }
}
