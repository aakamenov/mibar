use std::{time::Duration, any::Any};
use tokio::time::sleep;

use mibar_core::{
    widget::{
        text::{self, Text},
        Element, Widget, SizeConstraints
    },
    Size, Rect, DrawCtx, LayoutCtx, Id,
    Context, TypedId, StateHandle, Task
};

use crate::sys_info::Date;

pub struct DateTime {
    style: Option<text::StyleFn>
}

pub struct DateTimeWidget;

pub struct State {
    text: TypedId<Text>
}

impl DateTime {
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

fn get_time() -> (String, i32) {
    const UTC_OFFSET: u8 = 2;
    const WEEKDAYS: &[&str] = &["Sun", "Mon", "Tue", "Wen", "Thu", "Fri", "Sat"];

    let Some(date) = Date::now_with_offset(UTC_OFFSET) else {
        // 10 seconds is arbitrary here
        return (String::new(), 10);
    };

    let date_string = format!(
        "{:02}:{:02} {} {}/{}",
        date.hours,
        date.minutes,
        WEEKDAYS[date.day_of_week as usize],
        date.day_of_month,
        date.month + 1
    );

    (date_string, 60 - date.seconds)
}

impl Element for DateTime {
    type Widget = DateTimeWidget;
    type Message = ();

    fn make_widget(self, id: Id, ctx: &mut Context) -> (
        Self::Widget,
        <Self::Widget as Widget>::State
    ) {
        
        let (date, remaining) = get_time();

        let text = match self.style {
            Some(style) => Text::new(date).style(style),
            None => Text::new(date),
        };
        
        let state = State {
            text: ctx.new_child(id, text)
        };

        let _ = ctx.ui.spawn(Task::new(id, async move {
            sleep(Duration::from_secs(remaining as u64)).await;
            get_time()
        }));

        (DateTimeWidget, state)
    }
}

impl Widget for DateTimeWidget {
    type State = State;

    fn layout(handle: StateHandle<Self::State>, ctx: &mut LayoutCtx, bounds: SizeConstraints) -> Size {
        ctx.layout(ctx.tree[handle].text, bounds)
    }

    fn task_result(handle: StateHandle<Self::State>, ctx: &mut Context, data: Box<dyn Any>) {
        let result = data.downcast::<(String, i32)>().unwrap();

        let remaining = result.1;
        let _ = ctx.ui.spawn(Task::new(handle, async move {
            sleep(Duration::from_secs(remaining as u64)).await;
            get_time()
        }));

        ctx.message(ctx.tree[handle].text, text::Message::SetText(result.0));
    }

    fn draw(handle: StateHandle<Self::State>, ctx: &mut DrawCtx, _layout: Rect) {
        ctx.draw(ctx.tree[handle].text);
    }
}
