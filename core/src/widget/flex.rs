use smallvec::SmallVec;

use crate::{
    geometry::{Size, Rect},
    draw::{Quad, QuadStyle},
    InitCtx,DrawCtx, LayoutCtx,
    UpdateCtx, Event, Id, StateHandle
};
use super::{
    SizeConstraints, Padding,
    Alignment, Axis, Element, Widget
};

pub type StyleFn = fn() -> QuadStyle;
 
pub struct Flex<F: FnOnce(&mut FlexBuilder)> {
    create: F,
    axis: Axis,
    main_alignment: Alignment,
    cross_alignment: Alignment,
    spacing: f32,
    padding: Padding,
    style: Option<StyleFn>
}

pub struct FlexBuilder<'a: 'b, 'b> {
    ctx: &'b mut InitCtx<'a>,
    children: SmallVec<[(Id, f32); 8]>
}

pub struct FlexWidget;

#[derive(Clone)]
pub struct State {
    children: SmallVec<[(Id, f32); 8]>,
    axis: Axis,
    main_alignment: Alignment,
    cross_alignment: Alignment,
    spacing: f32,
    padding: Padding,
    style: Option<StyleFn>
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
    pub fn padding(mut self, padding: impl Into<Padding>) -> Self {
        self.padding = padding.into();

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
    pub fn style(mut self, style: StyleFn) -> Self {
        self.style = Some(style);

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
            padding: Padding::ZERO,
            style: None
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
            children: SmallVec::new()
        };

        (self.create)(&mut builder);

        let state = State {
            children: builder.children,
            axis: self.axis,
            main_alignment: self.main_alignment,
            cross_alignment: self.cross_alignment,
            spacing: self.spacing,
            padding: self.padding,
            style: self.style
        };
        
        (FlexWidget, state)
    }
}

impl Widget for FlexWidget {
    type State = State;

    // Simplified version of the Flutter flex layout algorithm:
    // https://api.flutter.dev/flutter/widgets/Flex-class.html
    fn layout(handle: StateHandle<Self::State>, ctx: &mut LayoutCtx, bounds: SizeConstraints) -> Size {
        let state = &ctx.tree[handle];
        let padding = state.padding;
        let axis = state.axis;
        let per_item_spacing = state.spacing;
        let main_alignment = state.main_alignment;
        let cross_alignment = state.cross_alignment;
        let children_len = state.children.len();

        let layout_bounds = bounds.pad(padding);
        let spacing = per_item_spacing *
            children_len.saturating_sub(1) as f32;

        let max_cross = axis.cross(layout_bounds.max);
        let mut cross = axis.cross(layout_bounds.min);
        let mut total_main = 0f32;

        let mut available = axis.main(layout_bounds.max) - spacing;
        let mut total_flex = 0f32;

        // Layout non-flex children first i.e those with flex factor == 0
        for i in 0..children_len {
            let (child, flex) = ctx.tree[handle].children[i];
            total_flex += flex;

            if flex.abs() > 0f32 {
                continue;
            }

            let (width, height) = axis.main_and_cross(available, max_cross);
            let widget_bounds = SizeConstraints::new(
                Size::ZERO,
                Size::new(width, height)
            );

            let size = ctx.layout(child, widget_bounds);

            let main_cross = axis.main_and_cross_size(size);
            available -= main_cross.0;
            total_main += main_cross.0;
            cross = cross.max(main_cross.1);
        }

        if total_flex > 0f32 {
            let available = available.max(0f32);

            // Layout flex children i.e those with flex factor > 0
            for i in 0..children_len {
                let (child, flex) = ctx.tree[handle].children[i];

                if flex <= 0f32 {
                    continue;
                }

                let max_main = available * flex / total_flex;
                let min_main = if max_main.is_infinite() {
                    0.0
                } else {
                    max_main
                };

                let (min_width, min_height) = axis.main_and_cross(
                    min_main,
                    axis.cross(layout_bounds.min)
                );

                let (max_width, max_height) = axis.main_and_cross(
                    max_main,
                    max_cross
                );

                let widget_bounds = SizeConstraints::new(
                    Size::new(min_width, min_height),
                    Size::new(max_width, max_height)
                );

                let size = ctx.layout(child, widget_bounds);

                let main_cross = axis.main_and_cross_size(size);
                total_main += main_cross.0;
                cross = cross.max(main_cross.1);
            }
        }

        let mut main = match main_alignment {
            Alignment::Start => {
                let (main_padding, _) = axis.main_and_cross(
                    padding.left,
                    padding.top
                );

                main_padding  
            }
            Alignment::Center => (axis.main(layout_bounds.max) -
                spacing -
                total_main) /
                2f32,
            Alignment::End => axis.main(layout_bounds.max) -
                spacing -
                total_main
        };

        // Position children
        for i in 0..children_len {
            let child = ctx.tree[handle].children[i].0;

            if i > 0 {
                main += per_item_spacing;
            }

            let origin = axis.main_and_cross(main, padding.top);

            let rect = ctx.position(child, |rect| {
                rect.set_origin(origin);
                cross_alignment.align(rect, cross, axis.flip());
            });

            main += axis.main(rect.size());
        }

        let (width, height) = axis.main_and_cross(
            main - padding.right,
            cross
        );
        let size = layout_bounds.constrain(Size::new(width, height));

        Size::new(
            size.width + (padding.horizontal()),
            size.height + (padding.vertical())
        )
    }

    fn draw(handle: StateHandle<Self::State>, ctx: &mut DrawCtx, rect: Rect) {
        let state = &ctx.tree[handle];
        let style = state.style;
        let children_len = state.children.len();

        if let Some(style) = style {
            ctx.renderer.fill_quad(Quad {
                rect,
                style: style()
            });
        }

        for i in 0..children_len {
            ctx.draw(ctx.tree[handle].children[i].0);
        }
    }

    fn event(handle: StateHandle<Self::State>, ctx: &mut UpdateCtx, event: &Event) {
        let children_len = ctx.tree[handle].children.len();

        for i in 0..children_len {
            ctx.event(ctx.tree[handle].children[i].0, event);
        }
    }
}
