use mibar_core::{
    widget::{
        size_constraints::SizeConstraints,
        Element, Widget
    },
    Size, Rect, InitCtx, DrawCtx, LayoutCtx, UpdateCtx,
    ValueSender, TextInfo, Color, Quad, Background,
    Weight
};

use tokio::{
    time::{Duration, interval},
    task::JoinHandle
};

use crate::sys_info::battery;

// TODO: Make these parameters
const UPDATE_INTERVAL: Duration = Duration::from_secs(30);
const DEVICE: &str = "BAT0";

const BODY_SIZE: Size = Size::new(30f32, 16f32);
const TERMINAL_SIZE: Size = Size::new(4f32, 9f32);
const BODY_RADIUS: f32 = 2f32;
const TEXT_SIZE: f32 = 12f32;

pub type StyleFn = fn(capacity: u8) -> Style;

pub struct Battery {
    style: StyleFn
}

pub struct BatteryWidget;

pub struct State {
    info: Option<BatteryInfo>,
    text_info: TextInfo,
    text_size: Size,
    style: StyleFn,
    handle: JoinHandle<()>
}

#[derive(Clone, Debug)]
pub struct Style {
    pub body: Color,
    pub background: Background,
    pub text: Color
}

#[derive(Clone, Copy, PartialEq, Debug)]
struct BatteryInfo {
    capacity: u8,
    status: battery::Status
}

impl Battery {
    #[inline]
    pub fn new(style: StyleFn) -> Self {
        Self { style }
    }
}

impl Element for Battery {
    type Widget = BatteryWidget;
    type Message = ();

    fn make_widget(self, ctx: &mut InitCtx) -> (
        Self::Widget,
        <Self::Widget as Widget>::State
    ) {
        let handle = ctx.task_with_sender(|sender: ValueSender<Option<BatteryInfo>>| {
            async move {
                let mut interval = interval(UPDATE_INTERVAL);

                loop {
                    interval.tick().await;

                    let (capacity, status) = tokio::join! {
                        battery::capacity(DEVICE),
                        battery::status(DEVICE)
                    };

                    if let Err(err) = capacity {
                        eprintln!("Error reading battery capacity: {}", err);
                        sender.send(None).await;

                        continue;
                    }

                    if let Err(err) = status {
                        eprintln!("Error reading battery status: {}", err);
                        sender.send(None).await;

                        continue;
                    }

                    let info = BatteryInfo {
                        capacity: capacity.unwrap(),
                        status: status.unwrap()
                    };

                    sender.send(Some(info)).await;
                }
            }
        });

        let mut font = ctx.theme().font;
        font.weight = Weight::BOLD;

        let state = State {
            info: None,
            style: self.style,
            text_info: TextInfo::new("0", TEXT_SIZE)
                .with_font(font),
            text_size: Size::ZERO,
            handle
        };

        (BatteryWidget, state)
    }
}

impl Widget for BatteryWidget {
    type State = State;

    fn layout(state: &mut Self::State, ctx: &mut LayoutCtx, _bounds: SizeConstraints) -> Size {
        match state.info {
            Some(_) => {
                state.text_size = ctx.measure_text(&state.text_info, BODY_SIZE);

                let mut size = BODY_SIZE;
                size.width += TERMINAL_SIZE.width;

                size
            },
            None => Size::ZERO
        }
    }

    fn task_result(
        state: &mut Self::State,
        ctx: &mut UpdateCtx,
        data: Box<dyn std::any::Any>
    ) {
        let info = *data.downcast::<Option<BatteryInfo>>().unwrap();

        if state.info != info {
            if let Some(info) = info {
                match info.status {
                    battery::Status::Charging =>
                        state.text_info.text = "󱐋".into(),
                    battery::Status::Full | battery::Status::Discharging =>
                        state.text_info.text = info.capacity.to_string()
                }
            }

            state.info = info;
            ctx.request_layout();
        }
    }

    fn draw(state: &mut Self::State, ctx: &mut DrawCtx) {
        let Some(info) = state.info else {
            return;
        };

        let style = (state.style)(info.capacity);

        let mut body = ctx.layout();
        body.set_size(BODY_SIZE);

        ctx.renderer.fill_quad(
            Quad::rounded(body, Color::TRANSPARENT, BODY_RADIUS)
                .with_border(2f32, style.body)
        );

        let body_center = body.center();
        let terminal = Rect::new(
            body.x + body.width,
            body_center.y - (TERMINAL_SIZE.height / 2f32),
            TERMINAL_SIZE.width,
            TERMINAL_SIZE.height
        );

        ctx.renderer.fill_quad(
            Quad::rounded(terminal, style.body, BODY_RADIUS)
        );

        let mut fill = body.shrink(2f32);
        fill.width = (fill.width * info.capacity as f32) / 100f32;

        ctx.renderer.fill_quad(Quad::new(fill, style.background));

        let mut text_rect = Rect::from_size(state.text_size);
        text_rect.x = body_center.x - (text_rect.width / 2f32);
        text_rect.y = body_center.y - (text_rect.height / 2f32);

        ctx.renderer.fill_text(
            &state.text_info,
            text_rect,
            style.text
        );
    }

    fn destroy(state: Self::State) {
        state.handle.abort();
    }
}
