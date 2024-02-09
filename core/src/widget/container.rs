use std::marker::PhantomData;

use crate::{
    DrawCtx, LayoutCtx, Context, TypedId, Event,
    StateHandle, Size, Rect, Quad, QuadStyle, Color, Id
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

pub struct Container<T: Element> {
    child: T,
    style: Option<StyleFn>,
    padding: Padding,
    width: Length,
    height: Length,
    horizontal_align: Alignment,
    vertical_align: Alignment
}

pub struct ContainerWidget<T: Element> {
    data: PhantomData<T>
}

pub struct State<T: Element> {
    child: TypedId<T>,
    style: Option<StyleFn>,
    padding: Padding,
    width: Length,
    height: Length,
    horizontal_align: Alignment,
    vertical_align: Alignment
}

impl<T: Element> Container<T> {
    #[inline]
    pub fn new(child: T) -> Self {
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

impl<T: Element + 'static> Element for Container<T> {
    type Widget = ContainerWidget<T>;

    fn make_state(self, id: Id, ctx: &mut Context) -> <Self::Widget as Widget>::State {
        State {
            child: ctx.new_child(id, self.child),
            style: self.style,
            padding: self.padding,
            width: self.width,
            height: self.height,
            horizontal_align: self.horizontal_align,
            vertical_align: self.vertical_align
        }
    }
}

impl<T: Element + 'static> Widget for ContainerWidget<T> {
    type State = State<T>;

    fn layout(
        handle: StateHandle<Self::State>,
        ctx: &mut LayoutCtx,
        bounds: SizeConstraints
    ) -> Size {
        let state = &ctx.tree[handle];

        layout_child(
            ctx,
            state.child.into(),
            bounds,
            state.padding,
            state.width,
            state.height,
            state.horizontal_align,
            state.vertical_align
        )
    }

    fn event(handle: StateHandle<Self::State>, ctx: &mut Context, event: &Event) {
        ctx.event(ctx.tree[handle].child, event);
    }

    fn draw(handle: StateHandle<Self::State>, ctx: &mut DrawCtx, rect: Rect) {
        let state = &ctx.tree[handle];

        if let Some(style) = state.style {
            let style = style();
            ctx.renderer.fill_quad(Quad { rect, style: style.quad });

            if let Some(color) = style.text_color {
                ctx.ui.theme().push_text_color(color);
                ctx.draw(state.child);
                ctx.ui.theme().pop_text_color();
            } else {
                ctx.draw(state.child);
            }
        } else {
            ctx.draw(state.child);
        }
    }
}

#[inline(always)]
pub fn layout_child(
    ctx: &mut LayoutCtx,
    child: Id,
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

impl<T: Element + 'static> TypedId<Container<T>> {
    #[inline]
    pub fn child(self, ctx: &mut Context) -> TypedId<T> {
        ctx.tree[self].child
    }
}

impl<T: Element> Default for ContainerWidget<T> {
    fn default() -> Self {
        Self { data: PhantomData }
    }
}
