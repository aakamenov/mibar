use std::{rc::Rc, marker::PhantomData, ops::Deref};

use smallvec::SmallVec;

use crate::{
    geometry::{Size, Rect},
    draw::{Quad, QuadStyle},
    reactive::{
        reactive_list::{ReactiveList, UniqueKey, ListOp},
        event_emitter::EventHandler
    },
    DrawCtx, LayoutCtx, Context, Event, Id, TypedId, StateHandle
};
use super::{
    SizeConstraints, Padding, FlexElementTuple,
    Alignment, Axis, Element, Widget
};

pub type StyleFn = fn() -> QuadStyle;
pub type CreateChildFn<T> = dyn Fn(DynamicFlexBuilder, &T) -> (Id, f32);

pub struct Flex {
    axis: Axis,
    main_alignment: Alignment,
    cross_alignment: Alignment,
    spacing: f32,
    padding: Padding,
    style: Option<StyleFn>
}

pub struct StaticFlex<T: FlexElementTuple> {
    children: T,
    params: Flex
}

pub struct DynamicFlex<T: UniqueKey> {
    create: Rc<CreateChildFn<T>>,
    params: Flex
}

pub struct DynamicFlexBuilder<'a: 'b, 'b> {
    id: Id,
    ctx: &'b mut Context<'a>
}

pub struct FlexWidget<T> {
    data: PhantomData<T>
}

pub struct State<T> {
    children: SmallVec<[(Id, f32); 8]>,
    axis: Axis,
    main_alignment: Alignment,
    cross_alignment: Alignment,
    spacing: f32,
    padding: Padding,
    style: Option<StyleFn>,
    create_child: Rc<CreateChildFn<T>>
}

impl Flex {
    #[inline]
    pub fn new(axis: Axis) -> Self {
        Self {
            axis,
            main_alignment: Alignment::Start,
            cross_alignment: Alignment::Center,
            spacing: 0f32,
            padding: Padding::ZERO,
            style: None
        }
    }

    #[inline]
    pub fn row() -> Self {
        Self::new(Axis::Horizontal)
    }

    #[inline]
    pub fn column() -> Self {
        Self::new(Axis::Vertical)
    }

    #[inline]
    pub fn build<T: FlexElementTuple>(self, children: T) -> StaticFlex<T> {
        StaticFlex { children, params: self }
    }

    #[inline]
    pub fn bind<T: UniqueKey + 'static>(
        self,
        ctx: &mut Context,
        list: &ReactiveList<T>,
        create: impl Fn(DynamicFlexBuilder, &T) -> (Id, f32) + 'static
    ) -> TypedId<DynamicFlex<T>> {
        let id = DynamicFlex { create: Rc::new(create), params: self }.make(ctx);
        list.subscribe(ctx, id);

        id
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
}

impl<'a: 'b, 'b> DynamicFlexBuilder<'a, 'b> {
    #[inline]
    pub fn non_flex(self, child: impl Element) -> (Id, f32) {
        self.flex(child, 0f32)
    }

    #[inline]
    pub fn flex(self, child: impl Element, flex: f32) -> (Id, f32) {
        let id = self.ctx.new_child(self.id, child);

        (id.into(), flex)
    }
}

impl<T: FlexElementTuple> Element for StaticFlex<T> {
    type Widget = FlexWidget<()>;

    fn make_state(self, id: Id, ctx: &mut Context) -> <Self::Widget as Widget>::State {
        let mut children = SmallVec::new();
        self.children.make(ctx, id, &mut children);

        State {
            children,
            axis: self.params.axis,
            main_alignment: self.params.main_alignment,
            cross_alignment: self.params.cross_alignment,
            spacing: self.params.spacing,
            padding: self.params.padding,
            style: self.params.style,
            create_child: Rc::new(|_, _| { unreachable!() })
        }
    }
}

impl<T: UniqueKey + 'static> Element for DynamicFlex<T> {
    type Widget = FlexWidget<T>;

    fn make_state(self, _id: Id, _ctx: &mut Context) -> <Self::Widget as Widget>::State {
        State {
            children: SmallVec::new(),
            axis: self.params.axis,
            main_alignment: self.params.main_alignment,
            cross_alignment: self.params.cross_alignment,
            spacing: self.params.spacing,
            padding: self.params.padding,
            style: self.params.style,
            create_child: self.create
        }
    }
}

impl<T: 'static> Widget for FlexWidget<T> {
    type State = State<T>;

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

    fn event(handle: StateHandle<Self::State>, ctx: &mut Context, event: &Event) {
        let children_len = ctx.tree[handle].children.len();

        for i in 0..children_len {
            ctx.event(ctx.tree[handle].children[i].0, event);
        }
    }
}

impl<T: UniqueKey + 'static> EventHandler<ListOp<T>> for FlexWidget<T> {
    fn handle(
        handle: StateHandle<Self::State>,
        ctx: &mut Context,
        event: &ListOp<T>
    ) {
        match event {
            ListOp::Init(items) => {
                let id = handle.id();
                let state = &mut ctx.tree[handle];
                let create_child = Rc::clone(&state.create_child);

                state.children.reserve(items.as_slice().len());

                for item in items.as_slice().deref() {
                    let result = create_child(DynamicFlexBuilder { id, ctx }, item);
                    ctx.tree[handle].children.push(result);
                }
            }
            ListOp::Changes(_) => { }
        }
    }
}

impl<T> Default for FlexWidget<T> {
    fn default() -> Self {
        Self { data: PhantomData }
    }
}
