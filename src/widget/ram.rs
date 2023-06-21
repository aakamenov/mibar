use crate::{
    geometry::Size,
    ui::{InitCtx, DrawCtx, LayoutCtx, UpdateCtx, Event},
    wayland::MouseEvent,
    renderer::Quad
};
use super::{
    size_constraints::SizeConstraints,
    Element, Widget
};

pub struct Ram;
pub struct RamWidget;
pub struct State;

impl Element for Ram {
    type Widget = RamWidget;
    type Message = ();

    fn make_widget(self, _ctx: &mut InitCtx) -> (
        Self::Widget,
        <Self::Widget as Widget>::State
    ) {
        (RamWidget, State)
    }
}

impl Widget for RamWidget {
    type State = State;

    fn layout(_state: &mut Self::State, _ctx: &mut LayoutCtx, bounds: SizeConstraints) -> Size {
        bounds.constrain(Size { width: 100f32, height: 20f32 })
    }

    fn draw(_state: &mut Self::State, ctx: &mut DrawCtx) {
        ctx.renderer.fill_quad(
            Quad::new(ctx.layout(), ctx.ui.theme.cold3)
        );
    }

    fn event(_state: &mut Self::State, ctx: &mut UpdateCtx, event: &Event) {
        match event {
            Event::Mouse(event) => {
                match *event {
                    MouseEvent::MousePress { pos, .. } => {
                        if ctx.layout().contains(pos) {
                            println!("mouse pressed");
                        }
                    },
                    MouseEvent::MouseRelease { pos, .. } => {
                        if ctx.layout().contains(pos) {
                            println!("mouse released");
                        }
                    },
                    MouseEvent::MouseMove(pos) => {
                        if ctx.layout().contains(pos) { }
                    }
                }
            }
        }
    }
}
