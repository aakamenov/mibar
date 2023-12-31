use std::{borrow::Borrow, marker::PhantomData};

use crate::{
    geometry::Size,
    ui::{InitCtx, DrawCtx, LayoutCtx, UpdateCtx, TypedId, Event},
    draw::{Quad, QuadStyle},
    Color, Id
};

use super::{
    Element, Widget, SizeConstraints,
    Padding, Length, Alignment, Axis
};

pub type StyleFn = fn() -> Style;

#[derive(Clone, PartialEq, Debug)]
pub struct Style {
    pub quad: QuadStyle,
    /// Overwrite the default text color for child widget.
    pub text_color: Option<Color>
}

pub struct Container<E: Element> {
    child: E,
    style: Option<StyleFn>,
    padding: Padding,
    width: Length,
    height: Length,
    horizontal_align: Alignment,
    vertical_align: Alignment
}

pub struct ContainerWidget<E: Element> {
    data: PhantomData<E>
}

pub struct State<E: Element> {
    child: TypedId<E>,
    style: Option<StyleFn>,
    padding: Padding,
    width: Length,
    height: Length,
    horizontal_align: Alignment,
    vertical_align: Alignment
}

impl<E: Element> Container<E> {
    #[inline]
    pub fn new(child: E) -> Self {
        Self {
            child,
            style: None,
            padding: Padding::from(0f32),
            width: Length::Fit,
            height: Length::Fit,
            horizontal_align: Alignment::Center,
            vertical_align: Alignment::Center
        }
    }

    #[inline]
    pub fn padding(mut self, padding: impl Into<Padding>) -> Self {
        self.padding = padding.into();

        self
    }

    #[inline]
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();

        self
    }

    #[inline]
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();

        self
    }

    #[inline]
    pub fn horizontal_alignment(mut self, alignment: Alignment) -> Self {
        self.horizontal_align = alignment;

        self
    }

    #[inline]
    pub fn vertical_alignment(mut self, alignment: Alignment) -> Self {
        self.vertical_align = alignment;

        self
    }

    #[inline]
    pub fn style(mut self, style: StyleFn) -> Self {
        self.style = Some(style);

        self
    }
}

impl<E: Element + 'static> Element for Container<E> {
    type Widget = ContainerWidget<E>;
    type Message = E::Message;

    fn make_widget(self, ctx: &mut InitCtx) -> (
        Self::Widget,
        <Self::Widget as Widget>::State
    ) {
        (
            ContainerWidget { data: PhantomData },
            State {
                child: ctx.new_child(self.child),
                style: self.style,
                padding: self.padding,
                width: self.width,
                height: self.height,
                horizontal_align: self.horizontal_align,
                vertical_align: self.vertical_align
            }
        )
    }

    fn message(
        state: &mut <Self::Widget as Widget>::State,
        ctx: &mut UpdateCtx,
        msg: Self::Message
    ) {
        ctx.message(&state.child, msg)
    }
}

impl<E: Element + 'static> Widget for ContainerWidget<E> {
    type State = State<E>;

    fn layout(
        state: &mut Self::State,
        ctx: &mut LayoutCtx,
        bounds: SizeConstraints
    ) -> Size {
        layout_child(
            ctx,
            state.child.borrow(),
            bounds,
            state.padding,
            state.width,
            state.height,
            state.horizontal_align,
            state.vertical_align
        )
    }

    fn event(state: &mut Self::State, ctx: &mut UpdateCtx, event: &Event) {
        ctx.event(&state.child, event);
    }

    fn draw(state: &mut Self::State, ctx: &mut DrawCtx) {
        if let Some(style) = state.style {
            let style = style();

            let rect = ctx.layout();
            ctx.renderer().fill_quad(Quad { rect, style: style.quad });

            if let Some(color) = style.text_color {
                ctx.theme().push_text_color(color);
                ctx.draw(&state.child);
                ctx.theme().pop_text_color();
            } else {
                ctx.draw(&state.child);
            }
        } else {
            ctx.draw(&state.child);
        }
    }
}

#[inline(always)]
pub fn layout_child(
    ctx: &mut LayoutCtx,
    child: &Id,
    bounds: SizeConstraints,
    padding: Padding,
    width: Length,
    height: Length,
    horizontal_align: Alignment,
    vertical_align: Alignment
) -> Size {
    let bounds = bounds.width(width).height(height);
    
    let layout_bounds = bounds.pad(padding).loosen();
    let child_size = ctx.layout(child, layout_bounds);

    let width = match width {
        Length::Fit => {
            child_size.width + padding.horizontal()
        }
        Length::Expand | Length::Fixed(_) => {
            bounds.max.width
        }
    };

    let height = match height {
        Length::Fit => {
            child_size.height + padding.vertical()
        }
        Length::Expand | Length::Fixed(_) => {
            bounds.max.height
        }
    };

    let size = bounds.constrain(Size::new(width, height));
    ctx.position(child, |rect| {
        horizontal_align.align(rect, size.width, Axis::Horizontal);
        vertical_align.align(rect, size.height, Axis::Vertical);
    });

    size
}
