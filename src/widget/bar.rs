use crate::{
    geometry::Size,
    positioner::Positioner,
    ui::DrawCtx
};
use super::{
    size_constraints::SizeConstraints,
    workspaces::Workspaces,
    date_time::DateTime,
    music::Music,
    cpu::Cpu,
    ram::Ram,
    flex::Flex,
    Widget
};

const PADDING: f32 = 6f32;
const SPACING: f32 = 10f32;

pub struct Bar {
    modules: Flex
}

impl Bar {
    pub fn new() -> Self {
        let left = Flex::row()
            .spacing(SPACING)
            .with_non_flex(Workspaces::new())
            .with_non_flex(DateTime::default());

        let middle = Flex::row()
            .spacing(SPACING)
            .with_non_flex(Music::default());

        let right = Flex::row()
            .spacing(SPACING)
            .with_non_flex(Cpu::default())
            .with_non_flex(Ram::default());

        Self {
            modules: Flex::row()
                .spacing(SPACING)
                .padding(PADDING)
                .with_flex(left, 1f32)
                .with_flex(middle, 2f32)
                .with_flex(right, 1f32)
        }
    }
}

impl Widget for Bar {
    fn layout(&mut self, bounds: SizeConstraints) -> Size {
        self.modules.layout(bounds)
    }

    fn draw(&mut self, ctx: &mut DrawCtx, positioner: Positioner) {
        self.modules.draw(ctx, positioner)
    }
}
