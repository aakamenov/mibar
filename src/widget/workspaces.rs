use tiny_skia::Color;

use crate::{
    geometry::{Size, Circle},
    positioner::Positioner,
    ui::{DrawCtx, LayoutCtx}
};
use super::{
    size_constraints::SizeConstraints,
    Widget
};

const WORKSPACE_COUNT: usize = 8;
const RADIUS: f32 = 8f32;
const SPACING: f32 = 3f32;

pub struct Workspaces {
    radius: f32
}

impl Workspaces {
    pub fn new() -> Self {
        Self { radius: RADIUS }
    }
}

impl Widget for Workspaces {
    fn layout(&mut self, ctx: &mut LayoutCtx, bounds: SizeConstraints) -> Size {
        let diameter = self.radius * 2f32;
        let diameter = diameter.clamp(bounds.min.height, bounds.max.height);
        self.radius = diameter / 2f32;

        let count = WORKSPACE_COUNT as f32;
        let spacing = (SPACING * count) - 1f32;
        let width = (diameter * count) + spacing;
        let size = bounds.constrain(Size {
            width,
            height: diameter
        });

        size
    }

    fn draw(&mut self, ctx: &mut DrawCtx, positioner: Positioner) {
        let bounds = positioner.bounds;
        let y = bounds.y + self.radius;
        let mut x = bounds.x + self.radius;

        for _ in 0..WORKSPACE_COUNT {
            let circle = Circle { x, y, radius: self.radius };
            ctx.fill_circle(circle, Color::BLACK);
            
            x += (self.radius * 2f32) + SPACING;
        }
    }
}
