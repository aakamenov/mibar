use std::{time::Duration, any::Any};
use tokio::time::sleep;

use crate::{
    geometry::Size,
    widget::{
        size_constraints::SizeConstraints,
        Element, Widget, text::{self, Text}
    },
    sys_info::Date,
    ui::{
        InitCtx, DrawCtx, LayoutCtx, UpdateCtx,
        TypedId
    }
};

pub struct DateTime;

pub struct DateTimeWidget;

pub struct State {
    text: TypedId<Text>
}

fn get_time() -> (String, i32) {
    const UTC_OFFSET: u8 = 3;
    const WEEKDAYS: &[&str] = &["Sun", "Mon", "Tue", "Wen", "Thu", "Fri", "Sat"];

    let Some(date) = Date::now_with_offset(UTC_OFFSET) else {
        // 10 seconds is arbitrary here
        return (String::new(), 10);
    };

    let date_string = format!(
        "{}:{} {} {}/{}",
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

    fn make_widget(self, ctx: &mut InitCtx) -> (
        Self::Widget,
        <Self::Widget as Widget>::State
    ) {
        
        let (date, remaining) = get_time();
        let state = State {
            text: ctx.new_child(Text::new(date))
        };

        ctx.task(async move {
            sleep(Duration::from_secs(remaining as u64)).await;
            get_time()
        });

        (DateTimeWidget, state)
    }
}

impl Widget for DateTimeWidget {
    type State = State;

    fn layout(state: &mut Self::State, ctx: &mut LayoutCtx, bounds: SizeConstraints) -> Size {
        ctx.layout(&state.text, bounds)
    }

    fn task_result(state: &mut Self::State, ctx: &mut UpdateCtx, data: Box<dyn Any>) {
        let result = data.downcast::<(String, i32)>().unwrap();

        let remaining = result.1;
        ctx.task(async move {
            sleep(Duration::from_secs(remaining as u64)).await;
            get_time()
        });

        ctx.message(&state.text, text::Message::SetText(result.0));
    }

    fn draw(state: &mut Self::State, ctx: &mut DrawCtx) {
        ctx.draw(&state.text);
    }
}
