pub mod size_constraints;
pub mod bar;
pub mod workspaces;
pub mod date_time;
pub mod cpu;
pub mod ram;
pub mod music;
pub mod flex;

use crate::{
    geometry::Size,
    positioner::Positioner,
    ui::DrawCtx
};
use size_constraints::SizeConstraints;

pub trait Widget {
    fn layout(&mut self, bounds: SizeConstraints) -> Size;
    fn draw(&mut self, ctx: &mut DrawCtx, positioner: Positioner);
}
