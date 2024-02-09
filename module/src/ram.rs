use mibar_core::{
    widget::{
        text::{self, Text},
        Element, Widget, SizeConstraints
    },
    DrawCtx, LayoutCtx, Context, Id, Task,
    ValueSender, TypedId, Size, Rect, StateHandle
};

use tokio::task::JoinHandle;

use crate::sys_info::{self, RamUsage};

pub struct Ram {
    style: Option<text::StyleFn>
}

#[derive(Default)]
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

    fn make_state(self, id: Id, ctx: &mut Context) -> <Self::Widget as Widget>::State {
        let mut rx = sys_info::RAM.subscribe(ctx.ui.runtime_handle());
        let task = Task::with_sender(id, |sender: ValueSender<RamUsage>| {
            async move {
                loop {
                    if let Ok(value) = rx.recv().await {
                        sender.send(*value);
                    }
                }
            }
        });

        let handle = ctx.ui.spawn(task);

        let text = format(RamUsage::default());
        let text = match self.style {
            Some(style) => Text::new(text).style(style),
            None => Text::new(text),
        };

        State {
            text: ctx.new_child(id, text),
            handle
        }
    }
}

impl Widget for RamWidget {
    type State = State;

    fn layout(handle: StateHandle<Self::State>, ctx: &mut LayoutCtx, bounds: SizeConstraints) -> Size {
        ctx.layout(ctx.tree[handle].text, bounds)
    }

    fn task_result(
        handle: StateHandle<Self::State>,
        ctx: &mut Context,
        data: Box<dyn std::any::Any>
    ) {
        let usage = *data.downcast::<RamUsage>().unwrap();
        let text = format(usage);

        let child = ctx.tree[handle].text;
        child.set_text(ctx, text);
    }

    fn draw(handle: StateHandle<Self::State>, ctx: &mut DrawCtx, _layout: Rect) {
        ctx.draw(ctx.tree[handle].text);
    }

    fn destroy(state: Self::State) {
        state.handle.abort();
    }
}
