use mibar_core::{
    widget::{SizeConstraints, Element, Widget},
    Size, Rect, DrawCtx, LayoutCtx, Context, Task,
    ValueSender, TextInfo, Color, Quad, Background,
    Weight, Font, StateHandle, Id
};

use tokio::{
    time::{Duration, interval},
    task::JoinHandle
};

use crate::sys_info::battery;

const BODY_SIZE: Size = Size::new(30f32, 16f32);
const TERMINAL_SIZE: Size = Size::new(4f32, 9f32);
const BODY_RADIUS: f32 = 2f32;
const TEXT_SIZE: f32 = 12f32;

pub type StyleFn = fn(capacity: u8) -> Style;

pub struct Battery {
    style: StyleFn,
    device: &'static str,
    interval: Duration
}

pub struct BatteryWidget;

pub struct State {
    info: BatteryInfoState,
    text: String,
    font: Font,
    text_dimensions: Size,
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
enum BatteryInfoState {
    InitialRead,
    Info {
        capacity: u8,
        status: battery::Status
    },
    Error
}

impl Battery {
    #[inline]
    pub fn new(device: &'static str, interval: Duration, style: StyleFn) -> Self {
        Self { style, device, interval }
    }
}

impl Element for Battery {
    type Widget = BatteryWidget;
    type Message = ();

    fn make_widget(self, id: Id, ctx: &mut Context) -> (
        Self::Widget,
        <Self::Widget as Widget>::State
    ) {
        let task = Task::with_sender(id, |sender: ValueSender<BatteryInfoState>| {
            async move {
                let mut interval = interval(self.interval);

                loop {
                    interval.tick().await;

                    let (capacity, status) = tokio::join! {
                        battery::capacity(self.device),
                        battery::status(self.device)
                    };

                    if let Err(err) = capacity {
                        eprintln!("Error reading battery capacity: {}", err);
                        sender.send(BatteryInfoState::Error);

                        continue;
                    }

                    if let Err(err) = status {
                        eprintln!("Error reading battery status: {}", err);
                        sender.send(BatteryInfoState::Error);

                        continue;
                    }

                    let info = BatteryInfoState::Info {
                        capacity: capacity.unwrap(),
                        status: status.unwrap()
                    };

                    sender.send(info);
                }
            }
        });

        let handle = ctx.ui.spawn(task);

        let mut font = ctx.ui.theme().font;
        font.weight = Weight::BOLD;

        let state = State {
            info: BatteryInfoState::InitialRead,
            style: self.style,
            text: String::from("0"),
            font,
            text_dimensions: Size::ZERO,
            handle
        };

        (BatteryWidget, state)
    }
}

impl Widget for BatteryWidget {
    type State = State;

    fn layout(handle: StateHandle<Self::State>, ctx: &mut LayoutCtx, bounds: SizeConstraints) -> Size {
        let state = &mut ctx.tree[handle];
        let size = match state.info {
            BatteryInfoState::Info { .. } | BatteryInfoState::Error => {
                let info = TextInfo::new(&state.text, TEXT_SIZE)
                    .with_font(state.font);

                state.text_dimensions = ctx.renderer.measure_text(&info, BODY_SIZE);

                let mut size = BODY_SIZE;
                size.width += TERMINAL_SIZE.width;

                size
            },
            BatteryInfoState::InitialRead => Size::ZERO
        };

        bounds.constrain(size)
    }

    fn task_result(
        handle: StateHandle<Self::State>,
        ctx: &mut Context,
        data: Box<dyn std::any::Any>
    ) {
        let info = *data.downcast::<BatteryInfoState>().unwrap();
        let state = &mut ctx.tree[handle];

        if state.info == info {
            return;
        }

        match info {
            BatteryInfoState::Info { capacity, status } => {
                match status {
                    battery::Status::Charging =>
                        state.text = "󱐋".into(),
                    battery::Status::Full | battery::Status::Discharging =>
                        state.text = capacity.to_string()
                }
            }
            BatteryInfoState::Error => {
                state.text = "N/A".into();
            }
            BatteryInfoState::InitialRead => unreachable!()
        }

        state.info = info;
        ctx.ui.request_layout();
    }

    fn draw(handle: StateHandle<Self::State>, ctx: &mut DrawCtx, layout: Rect) {
        let state = &ctx.tree[handle];

        if let BatteryInfoState::InitialRead = state.info {
            return;
        };

        let capacity = if let BatteryInfoState::Info { capacity, .. } = state.info {
            capacity
        } else {
            0
        };

        let style = (state.style)(capacity);

        let mut body = layout;
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
        fill.width = (fill.width * capacity as f32) / 100f32;

        ctx.renderer.fill_quad(Quad::new(fill, style.background));

        let mut text_rect = Rect::from_size(state.text_dimensions);
        text_rect.x = body_center.x - (text_rect.width / 2f32);
        text_rect.y = body_center.y - (text_rect.height / 2f32);

        let info = TextInfo::new(&state.text, TEXT_SIZE)
            .with_font(state.font);

        ctx.renderer.fill_text(
            &info,
            text_rect,
            style.text
        );
    }

    fn destroy(state: Self::State) {
        state.handle.abort();
    }
}
