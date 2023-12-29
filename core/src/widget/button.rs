use std::marker::PhantomData;

use crate::{
    geometry::Size,
    ui::{InitCtx, DrawCtx, LayoutCtx, UpdateCtx, TypedId, Event},
    draw::{Quad, QuadStyle},
    MouseEvent, MouseButton
};
use super::{
    text::Text,
    Element, Widget, SizeConstraints,
    Padding, Length, Alignment, Axis
};

pub type StyleFn = fn(ButtonState) -> QuadStyle;
pub type OnClickFn<E> = Box<dyn Fn(&mut UpdateCtx, &TypedId<E>)>;
pub type OnStateChangeFn<E> = fn(
    state: &mut State<E>,
    ctx: &mut UpdateCtx,
    btn_state: ButtonState
);

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ButtonState {
    Normal,
    Hovered,
    Active
}

pub struct Button<E: Element> {
    child: E,
    on_click: OnClickFn<E>,
    on_change: Option<OnStateChangeFn<E>>,
    style: Option<StyleFn>,
    padding: Padding,
    width: Length,
    height: Length
}

pub struct ButtonWidget<E: Element> {
    data: PhantomData<E>
}

pub struct State<E: Element> {
    child: TypedId<E>,
    on_click: OnClickFn<E>,
    on_change: Option<OnStateChangeFn<E>>,
    style: Option<StyleFn>,
    padding: Padding,
    width: Length,
    height: Length,
    is_hovered: bool,
    is_active: bool
}

impl Button<Text> {
    #[inline]
    pub fn new(
        text: impl Into<String>,
        on_click: impl Fn(&mut UpdateCtx, &TypedId<Text>) + 'static
    ) -> Self {
        Self::with_child(Text::new(text), on_click)
    }
}

impl<E: Element> Button<E> {
    #[inline]
    pub fn with_child(
        child: E,
        on_click: impl Fn(&mut UpdateCtx, &TypedId<E>) + 'static
    ) -> Self {
        Self {
            child,
            on_click: Box::new(on_click),
            on_change: None,
            style: None,
            padding: Padding::from(4f32),
            width: Length::Fit,
            height: Length::Fit
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
    pub fn style(mut self, style: StyleFn) -> Self {
        self.style = Some(style);

        self
    }

    #[inline]
    pub fn on_state_change(mut self, handler: OnStateChangeFn<E>) -> Self {
        self.on_change = Some(handler);

        self
    }
}

impl<E: Element + 'static> Element for Button<E> {
    type Widget = ButtonWidget<E>;
    type Message = E::Message;

    fn make_widget(self, ctx: &mut InitCtx) -> (
        Self::Widget,
        <Self::Widget as Widget>::State
    ) {
        (
            ButtonWidget { data: PhantomData },
            State {
                child: ctx.new_child(self.child),
                on_click: self.on_click,
                on_change: self.on_change,
                style: self.style,
                is_hovered: false,
                is_active: false,
                padding: self.padding,
                width: self.width,
                height: self.height
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

impl<E: Element + 'static> Widget for ButtonWidget<E> {
    type State = State<E>;

    fn layout(
        state: &mut Self::State,
        ctx: &mut LayoutCtx,
        bounds: SizeConstraints
    ) -> Size {
        let bounds = bounds.width(state.width).height(state.height);
        
        let layout_bounds = bounds.pad(state.padding).loosen();
        let child_size = ctx.layout(&state.child, layout_bounds);

        let width = match state.width {
            Length::Fit => {
                child_size.width + state.padding.horizontal()
            }
            Length::Expand | Length::Fixed(_) => {
                bounds.max.width
            }
        };

        let height = match state.height {
            Length::Fit => {
                child_size.height + state.padding.vertical()
            }
            Length::Expand | Length::Fixed(_) => {
                bounds.max.height
            }
        };

        let size = bounds.constrain(Size::new(width, height));
        ctx.position(&state.child, |rect| {
            Alignment::Center.align(rect, size.width, Axis::Horizontal);
            Alignment::Center.align(rect, size.height, Axis::Vertical);
        });

        size
    }

    fn event(state: &mut Self::State, ctx: &mut UpdateCtx, event: &Event) {
        let old_state = state.current_state();

        match event {
            Event::Mouse(event) => match event {
                MouseEvent::MouseMove(pos) => {
                    if ctx.layout().contains(*pos) {
                        if !state.is_hovered {
                            ctx.request_redraw();
                        }

                        state.is_hovered = true;
                    } else if state.is_hovered {
                        state.is_hovered = false;
                        state.is_active = false;

                        ctx.request_redraw();
                    }
                }
                MouseEvent::MousePress { pos, button }
                    if matches!(button, MouseButton::Left) =>
                {
                    if ctx.layout().contains(*pos) {
                        state.is_active = true;
                        ctx.request_redraw();
                    }
                }
                MouseEvent::MouseRelease { button, .. }
                    if matches!(button, MouseButton::Left) =>
                {
                    if state.is_active {
                        (state.on_click)(ctx, &state.child);

                        ctx.request_redraw();
                        state.is_active = false;
                    }
                }
                _ => { }
            }
        }

        if let Some(on_change) = state.on_change {
            let new_state = state.current_state();

            if old_state != new_state {
                (on_change)(state, ctx, new_state);
            }      
        }   
    }

    fn draw(state: &mut Self::State, ctx: &mut DrawCtx) {
        let style = state.style.unwrap_or(ctx.theme().button);
        let style = style(state.current_state());

        let rect = ctx.layout();
        ctx.renderer().fill_quad(Quad { rect, style });

        ctx.draw(&state.child);
    }
}

impl<E: Element> State<E> {
    #[inline]
    fn current_state(&self) -> ButtonState {
        if self.is_active {
            ButtonState::Active
        } else if self.is_hovered {
            ButtonState::Hovered
        } else {
            ButtonState::Normal
        }
    }
}
