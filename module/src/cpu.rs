use mibar_core::{
    widget::{
        text::{self, Text},
        SizeConstraints, Element, Widget
    },
    InitCtx, DrawCtx, LayoutCtx, UpdateCtx,
    ValueSender, TypedId, Size, Rect, StateHandle
};

use crate::sys_info;

use tokio::task::JoinHandle;

pub struct Cpu {
    style: Option<text::StyleFn>
}

pub struct CpuWidget;

pub struct State {
    text: TypedId<Text>,
    handle: JoinHandle<()>
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
        let mut rx = sys_info::CPU.subscribe(ctx.ui.runtime_handle());
        let handle = ctx.task_with_sender(|sender: ValueSender<f64>| {
            async move {
                loop {
                    if let Ok(value) = rx.recv().await {
                        sender.send(*value);
                    }
                }
            }
        });

        let text = format(0f64);
        let text = match self.style {
            Some(style) => Text::new(text).style(style),
            None => Text::new(text),
        };
        
        let state = State {
            text: ctx.new_child(text),
            handle
        };

        (CpuWidget, state)
    }
}

impl Widget for CpuWidget {
    type State = State;

    fn layout(handle: StateHandle<Self::State>, ctx: &mut LayoutCtx, bounds: SizeConstraints) -> Size {
        ctx.layout(ctx.tree[handle].text, bounds)
    }

    fn task_result(
        handle: StateHandle<Self::State>,
        ctx: &mut UpdateCtx,
        data: Box<dyn std::any::Any>
    ) {
        let usage = *data.downcast::<f64>().unwrap();
        let text = format(usage);

        ctx.message(ctx.tree[handle].text, text::Message::SetText(text));
    }

    fn draw(handle: StateHandle<Self::State>, ctx: &mut DrawCtx, _layout: Rect) {
        ctx.draw(ctx.tree[handle].text);
    }

    fn destroy(state: Self::State) {
        state.handle.abort();
    }
}
