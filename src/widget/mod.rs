pub mod size_constraints;
pub mod workspaces;
pub mod date_time;
pub mod cpu;
pub mod ram;
pub mod music;
pub mod flex;

use crate::{
    geometry::Size,
    ui::{DrawCtx, LayoutCtx, UpdateCtx, Event, ChildWidgets}
};
use size_constraints::SizeConstraints;

pub trait Widget {
    fn children(&self) -> ChildWidgets {
        ChildWidgets::default()
    }
    
    fn layout(&mut self, ctx: &mut LayoutCtx, bounds: SizeConstraints) -> Size;
    fn draw(&mut self, ctx: &mut DrawCtx);
    fn event(&mut self, _ctx: &mut UpdateCtx, _event: &Event) { }
}
