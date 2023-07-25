use crate::{
    geometry::{Size, Rect},
    renderer::Quad,
    ui::{
        InitCtx,DrawCtx, LayoutCtx,
        UpdateCtx, Event, Id
    }
};
use super::{
    size_constraints::SizeConstraints,
    Element, Widget
};

pub struct Flex<F: FnOnce(&mut FlexBuilder)> {
    create: F,
    axis: Axis,
    main_alignment: Alignment,
    cross_alignment: Alignment,
    spacing: f32,
    padding: f32
}

pub struct FlexBuilder<'a: 'b, 'b> {
    ctx: &'b mut InitCtx<'a>,
    children: Vec<(Id, f32)>
}

pub struct FlexWidget;

pub struct State {
    children: Vec<(Id, f32)>,
    axis: Axis,
    main_alignment: Alignment,
    cross_alignment: Alignment,
    spacing: f32,
    padding: f32
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Alignment {
    Start,
    Center,
    End
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Axis {
    Horizontal,
    Vertical
}

impl<F: FnOnce(&mut FlexBuilder)> Flex<F> {
    #[inline]
    pub fn row(create: F) -> Self {
        Self::new(create, Axis::Horizontal)
    }

    #[inline]
    pub fn column(create: F) -> Self {
        Self::new(create, Axis::Vertical)
    }

    #[inline]
    pub fn padding(mut self, padding: f32) -> Self {
        self.padding = padding;

        self
    }

    #[inline]
    pub fn spacing(mut self, spacing: f32) -> Self {
        self.spacing = spacing;

        self
    }

    #[inline]
    pub fn cross_alignment(mut self, alignment: Alignment) -> Self {
        self.cross_alignment = alignment;

        self
    }

    #[inline]
    pub fn main_alignment(mut self, alignment: Alignment) -> Self {
        self.main_alignment = alignment;

        self
    }

    #[inline]
    fn new(create: F, axis: Axis) -> Self {
        Self {
            create,
            axis,
            main_alignment: Alignment::Start,
            cross_alignment: Alignment::Center,
            spacing: 0f32,
            padding: 0f32
        }
    }
}

impl<'a: 'b, 'b> FlexBuilder<'a, 'b> {
    #[inline]
    pub fn add_non_flex(
        &mut self,
        child: impl Element
    ) {
        self.add_flex(child, 0f32);
    }

    #[inline]
    pub fn add_flex(
        &mut self,
        child: impl Element,
        flex: f32
    ) {
        let id = self.ctx.new_child(child);
        self.children.push((id.into(), flex));
    }

    #[inline]
    pub fn reserve(&mut self, additional: usize) {
        self.children.reserve(additional);
    }

    #[inline]
    pub fn reserve_exact(&mut self, additional: usize) {
        self.children.reserve_exact(additional);
    }
}

impl<F: FnOnce(&mut FlexBuilder)> Element for Flex<F> {
    type Widget = FlexWidget;
    type Message = ();

    fn make_widget(self, ctx: &mut InitCtx) -> (
        Self::Widget,
        <Self::Widget as Widget>::State
    ) {
        let mut builder = FlexBuilder {
            ctx,
            children: Vec::new()
        };

        (self.create)(&mut builder);

        let state = State {
            children: builder.children,
            axis: self.axis,
            main_alignment: self.main_alignment,
            cross_alignment: self.cross_alignment,
            spacing: self.spacing,
            padding: self.padding,
        };
        
        (FlexWidget, state)
    }
}

impl Widget for FlexWidget {
    type State = State;

    // Simplified version of the Flutter flex layout algorithm:
    // https://api.flutter.dev/flutter/widgets/Flex-class.html
    fn layout(state: &mut Self::State, ctx: &mut LayoutCtx, bounds: SizeConstraints) -> Size {
        let layout_bounds = bounds.shrink(
            Size::new(state.padding * 2f32, state.padding * 2f32)
        );
        let spacing = state.spacing *
            state.children.len().saturating_sub(1) as f32;

        let max_cross = state.axis.cross(layout_bounds.max);
        let mut cross = state.axis.cross(layout_bounds.min);
        let mut total_main = 0f32;

        let mut available = state.axis.main(layout_bounds.max) - spacing;
        let mut total_flex = 0f32;

        // Layout non-flex children first i.e those with flex factor == 0
        for (child, flex) in &state.children {
            total_flex += *flex;

            if flex.abs() > 0f32 {
                continue;
            }

            let (width, height) = state.axis.main_and_cross(available, max_cross);
            let widget_bounds = SizeConstraints::new(
                Size::ZERO,
                Size::new(width, height)
            );

            let size = ctx.layout(child, widget_bounds);

            let main_cross = state.axis.main_and_cross_size(size);
            available -= main_cross.0;
            total_main += main_cross.0;
            cross = cross.max(main_cross.1);
        }

        if total_flex > 0f32 {
            let available = available.max(0f32);

            // Layout flex children i.e those with flex factor > 0
            for (child, flex) in &state.children {
                if *flex <= 0f32 {
                    continue;
                }

                let max_main = available * *flex / total_flex;
                let min_main = if max_main.is_infinite() {
                    0.0
                } else {
                    max_main
                };

                let (min_width, min_height) = state.axis.main_and_cross(
                    min_main,
                    state.axis.cross(layout_bounds.min)
                );

                let (max_width, max_height) = state.axis.main_and_cross(
                    max_main,
                    max_cross
                );

                let widget_bounds = SizeConstraints::new(
                    Size::new(min_width, min_height),
                    Size::new(max_width, max_height)
                );

                let size = ctx.layout(child, widget_bounds);

                let main_cross = state.axis.main_and_cross_size(size);
                total_main += main_cross.0;
                cross = cross.max(main_cross.1);
            }
        }

        let mut main = match state.main_alignment {
            Alignment::Start => state.padding,
            Alignment::Center => (state.axis.main(layout_bounds.max) -
                spacing -
                total_main) /
                2f32,
            Alignment::End => state.axis.main(layout_bounds.max) -
                spacing -
                total_main
        };

        // Position children
        for (i, (child, _)) in state.children.iter().enumerate() {
            if i > 0 {
                main += state.spacing;
            }

            let origin = state.axis.main_and_cross(main, state.padding);

            let rect = ctx.position(child, |rect| {
                rect.set_origin(origin);
                state.cross_alignment.align(rect, cross, state.axis.flip());
            });

            main += state.axis.main(rect.size());
        }

        let (width, height) = state.axis.main_and_cross(main - state.padding, cross);
        let size = layout_bounds.constrain(Size::new(width, height));

        Size::new(
            size.width + (state.padding * 2f32),
            size.height + (state.padding * 2f32)
        )
    }

    fn draw(state: &mut Self::State, ctx: &mut DrawCtx) {
        ctx.renderer.fill_quad(Quad::new(ctx.layout(), ctx.theme().base));

        for (child, _) in &state.children {
            ctx.draw(child);
        }
    }

    fn event(state: &mut Self::State, ctx: &mut UpdateCtx, event: &Event) {
        for (child, _) in &state.children {
            ctx.event(child, event);
        }
    }
}

impl Axis {
    #[inline]
    pub fn flip(&self) -> Self {
        match self {
            Self::Horizontal => Self::Vertical,
            Self::Vertical => Self::Horizontal
        }
    }

    #[inline]
    pub fn main(&self, size: Size) -> f32 {
        match self {
            Self::Horizontal => size.width,
            Self::Vertical => size.height
        }
    }

    #[inline]
    pub fn cross(&self, size: Size) -> f32 {
        match self {
            Self::Horizontal => size.height,
            Self::Vertical => size.width
        }
    }

    #[inline]
    pub fn main_and_cross_size(&self, size: Size) -> (f32, f32) {
        match self {
            Self::Horizontal => (size.width, size.height),
            Self::Vertical => (size.height, size.width)
        }
    }

    #[inline]
    pub fn main_and_cross(&self, main: f32, cross: f32) -> (f32, f32) {
        match self {
            Self::Horizontal => (main, cross),
            Self::Vertical => (cross, main)
        }
    }
}

impl Alignment {
    fn align(&self, rect: &mut Rect, space: f32, axis: Axis) {
        let (value, size) = match axis {
            Axis::Horizontal => (&mut rect.x, rect.width),
            Axis::Vertical => (&mut rect.y, rect.height)
        };

        match self {
            Self::Start => { }
            Self::Center => *value += (space - size) / 2f32,
            Self::End => *value += space - size
        }
    }
}
