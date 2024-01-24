use std::{marker::PhantomData, rc::Rc};

use crate::{
    InitCtx, DrawCtx, LayoutCtx, UpdateCtx, TypedId, Event, Size,
    MouseEvent, MouseButton, Color, StateHandle, Quad, QuadStyle,
    Rect, Action, ActionId
};
use super::{
    container,
    Element, Widget, SizeConstraints, Text,
    Padding, Length, Alignment
};

pub type StyleFn = fn(ButtonState) -> Style;

#[derive(Clone, PartialEq, Debug)]
pub struct Style {
    pub quad: QuadStyle,
    /// Overwrite the default text color for child widget.
    pub text_color: Option<Color>
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ButtonState {
    Normal,
    Hovered,
    Active
}

pub struct Button<E: Element> {
    child: E,
    on_click: Action,
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
    on_click: ActionId,
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
        on_click: impl Fn(&mut UpdateCtx) + 'static
    ) -> Self {
        Self::with_child(Text::new(text), on_click)
    }
}

impl<E: Element> Button<E> {
    #[inline]
    pub fn with_child(
        child: E,
        on_click: impl Fn(&mut UpdateCtx) + 'static
    ) -> Self {
        Self {
            child,
            on_click: Rc::new(on_click),
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
}

impl<E: Element + 'static> Element for Button<E> {
    type Widget = ButtonWidget<E>;
    type Message = E::Message;

    fn make_widget(self, ctx: &mut InitCtx) -> (
        Self::Widget,
        <Self::Widget as Widget>::State
    ) {
        let on_click = ctx.register_action(self.on_click);

        (
            ButtonWidget { data: PhantomData },
            State {
                child: ctx.new_child(self.child),
                on_click,
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
        handle: StateHandle<<Self::Widget as Widget>::State>,
        ctx: &mut UpdateCtx,
        msg: Self::Message
    ) {
        let child = ctx.tree[handle].child;

        ctx.message(child, msg)
    }
}

impl<E: Element + 'static> Widget for ButtonWidget<E> {
    type State = State<E>;

    fn layout(
        handle: StateHandle<Self::State>,
        ctx: &mut LayoutCtx,
        bounds: SizeConstraints
    ) -> Size {
        let state = &ctx.tree[handle];

        container::layout_child(
            ctx,
            state.child.into(),
            bounds,
            state.padding,
            state.width,
            state.height,
            Alignment::Center,
            Alignment::Center
        )
    }

    fn event(handle: StateHandle<Self::State>, ctx: &mut UpdateCtx, event: &Event) {
        let layout = ctx.layout();
        let state = &mut ctx.tree[handle];

        match event {
            Event::Mouse(event) => match event {
                MouseEvent::MouseMove(pos) => {
                    if layout.contains(*pos) {
                        if !state.is_hovered {
                            ctx.ui.request_redraw();
                        }

                        state.is_hovered = true;
                    } else if state.is_hovered {
                        state.is_hovered = false;
                        state.is_active = false;

                        ctx.ui.request_redraw();
                    }
                }
                MouseEvent::MousePress { pos, button }
                    if matches!(button, MouseButton::Left) =>
                {
                    if layout.contains(*pos) {
                        state.is_active = true;
                        ctx.ui.request_redraw();
                    }
                }
                MouseEvent::MouseRelease { button, .. }
                    if matches!(button, MouseButton::Left) =>
                {
                    if state.is_active {
                        ctx.ui.request_redraw();
                        state.is_active = false;

                        let on_click = state.on_click;
                        ctx.execute(on_click);                        
                    }
                }
                _ => { }
            }
        }
    }

    fn draw(handle: StateHandle<Self::State>, ctx: &mut DrawCtx, rect: Rect) {
        let state = &ctx.tree[handle];
        let style = state.style.unwrap_or(ctx.ui.theme().button);
        let style = style(state.current_state());

        ctx.renderer.fill_quad(Quad { rect, style: style.quad });

        if let Some(color) = style.text_color {
            ctx.ui.theme().push_text_color(color);
            ctx.draw(state.child);
            ctx.ui.theme().pop_text_color();
        } else {
            ctx.draw(state.child);
        }
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
