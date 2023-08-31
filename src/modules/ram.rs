use crate::{
    geometry::Size,
    widget::{
        size_constraints::SizeConstraints,
        Element, Widget, text::{self, Text}
    },
    ui::{InitCtx, DrawCtx, LayoutCtx, ValueSender, TypedId},
    sys_info::{self, RamUsage}
};

use tokio::time::{Duration, interval};

const UPDATE_INTERVAL: Duration = Duration::from_millis(1000);

pub struct Ram;
pub struct RamWidget;
pub struct State {
    text: TypedId<Text>
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
        ctx.task_with_sender(|sender: ValueSender<RamUsage>| {
            async move {
                let mut interval = interval(UPDATE_INTERVAL);

                loop {
                    interval.tick().await;
                    sender.send(sys_info::ram_usage()).await;
                }
            }
        });

        let state = State {
            text: ctx.new_child(Text::new(format(RamUsage::default())))
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
        ctx: &mut crate::ui::UpdateCtx,
        data: Box<dyn std::any::Any>
    ) {
        let usage = *data.downcast::<RamUsage>().unwrap();
        let text = format(usage);

        ctx.message(&state.text, text::Message::SetText(text));
    }

    fn draw(state: &mut Self::State, ctx: &mut DrawCtx) {
        ctx.draw(&state.text);
    }
}
