use crate::{
    DrawCtx, LayoutCtx, Context, StateHandle, Id, TypedId,
    Size, Rect, Font, Color, TextInfo, LineHeight
};
use super::{SizeConstraints, Element, Widget};

pub type StyleFn = fn() -> Color;

pub struct Text {
    text: String,
    text_size: Option<f32>,
    font: Option<Font>,
    line_height: Option<LineHeight>,
    style: Option<StyleFn>
}

#[derive(Default)]
pub struct TextWidget;

#[derive(Debug)]
pub struct State {
    text: String,
    text_size: f32,
    font: Font,
    line_height: LineHeight,
    style: Option<StyleFn>
}

impl Text {
    #[inline]
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            text_size: None,
            font: None,
            line_height: None,
            style: None
        }
    }

    #[inline]
    pub fn text_size(mut self, size: f32) -> Self {
        self.text_size = Some(size);

        self
    }

    #[inline]
    pub fn line_height(mut self, line_height: LineHeight) -> Self {
        self.line_height = Some(line_height);

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

    fn make_state(self, _: Id, ctx: &mut Context) -> <Self::Widget as Widget>::State {
        let theme = ctx.ui.theme();

        State {
            text: self.text,
            text_size: self.text_size.unwrap_or(theme.font_size),
            font: self.font.unwrap_or(theme.font),
            line_height: self.line_height.unwrap_or_default(),
            style: self.style
        }
    }
}

impl Widget for TextWidget {
    type State = State;

    fn layout(
        handle: StateHandle<Self::State>,
        ctx: &mut LayoutCtx,
        bounds: SizeConstraints
    ) -> Size {
        let state = &ctx.tree[handle];
        let info = TextInfo {
            text: &state.text,
            size: state.text_size,
            line_height: state.line_height,
            font: state.font
        };

        bounds.constrain(ctx.renderer.measure_text(&info, bounds.max))
    }

    fn draw(handle: StateHandle<Self::State>, ctx: &mut DrawCtx, rect: Rect) {
        let state = &ctx.tree[handle];
        let color = if let Some(color_fn) = state.style {
            color_fn()
        } else {
            ctx.ui.theme().text_color()
        };
        
        let info = TextInfo {
            text: &state.text,
            size: state.text_size,
            line_height: state.line_height,
            font: state.font
        };

        ctx.renderer.fill_text(&info, rect, color);
    }
}

impl TypedId<Text> {
    pub fn set_text(self, ctx: &mut Context, text: impl Into<String>) {
        let text = text.into();
        let state = &mut ctx.tree[self];

        if text != state.text {
            state.text = text;
            ctx.ui.request_layout();
        }
    }

    pub fn set_style(self, ctx: &mut Context, style: StyleFn) {
        ctx.tree[self].style = Some(style);
        ctx.ui.request_redraw();
    }
}
