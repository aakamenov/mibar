use crate::{
    geometry::Size,
    ui::{InitCtx, DrawCtx, LayoutCtx, UpdateCtx},
    draw::TextInfo,
    theme::Font,
    color::Color
};
use super::{SizeConstraints, Element, Widget};

pub type StyleFn = fn() -> Color;

pub struct Text {
    text: String,
    text_size: Option<f32>,
    font: Option<Font>,
    style: Option<StyleFn>
}

pub struct TextWidget;

#[derive(Debug)]
pub enum Message {
    SetText(String),
    SetStyle(StyleFn)
}

#[derive(Debug)]
pub struct State {
    info: TextInfo,
    style: Option<StyleFn>
}

impl Text {
    #[inline]
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            text_size: None,
            font: None,
            style: None
        }
    }

    #[inline]
    pub fn text_size(mut self, size: f32) -> Self {
        self.text_size = Some(size);

        self
    }

    #[inline]
    pub fn font(mut self, font: Font) -> Self {
        self.font = Some(font);

        self
    }

    #[inline]
    pub fn style(mut self, style: StyleFn) -> Self {
        self.style = Some(style);

        self
    }
}

impl Element for Text {
    type Widget = TextWidget;
    type Message = Message;

    fn make_widget(self, ctx: &mut InitCtx) -> (
        Self::Widget,
        <Self::Widget as Widget>::State
    ) {
        let theme = ctx.theme();
        let info = TextInfo::new(
            self.text,
            self.text_size.unwrap_or(theme.font_size),
        ).with_font(
            self.font.unwrap_or(theme.font)
        );

        (
            TextWidget,
            State {
                info,
                style: self.style
            }
        )
    }

    fn message(
        state: &mut <Self::Widget as Widget>::State,
        ctx: &mut UpdateCtx,
        msg: Self::Message
    ) {
        match msg {
            Message::SetText(text) => {
                if text != state.info.text {
                    state.info.text = text;
                    ctx.request_layout();
                }
            }
            Message::SetStyle(style) => {
                state.style = Some(style);
                ctx.request_redraw();
            }
        }
    }
}

impl Widget for TextWidget {
    type State = State;

    fn layout(
        state: &mut Self::State,
        ctx: &mut LayoutCtx,
        bounds: SizeConstraints
    ) -> Size {
        bounds.constrain(ctx.measure_text(&state.info, bounds.max))
    }

    fn draw(state: &mut Self::State, ctx: &mut DrawCtx) {
        let style = state.style.unwrap_or(ctx.theme().text);
        let color = (style)();
        
        ctx.renderer.fill_text(&state.info, ctx.layout(), color);
    }
}
