use crate::{
    ui::{CreateCtx, Id},
    widget::{
        workspaces::Workspaces,
        date_time::DateTime,
        music::Music,
        cpu::Cpu,
        ram::Ram,
        flex::{Flex, Alignment}
    }
};

const PADDING: f32 = 6f32;
const SPACING: f32 = 10f32;

pub fn build(ctx: &mut CreateCtx) -> Id {
    let left = Flex::row()
        .spacing(SPACING)
        .with_non_flex(ctx.alloc(Workspaces::new()))
        .with_non_flex(ctx.alloc(DateTime::default()));

    let middle = Flex::row()
        .spacing(SPACING)
        .with_non_flex(ctx.alloc(Music::default()));

    let right = Flex::row()
        .spacing(SPACING)
        .main_alignment(Alignment::End)
        .with_non_flex(ctx.alloc(Cpu::default()))
        .with_non_flex(ctx.alloc(Ram::default()));

    let left = ctx.alloc(left);
    let middle = ctx.alloc(middle);
    let right = ctx.alloc(right);

    let root = Flex::row()
        .spacing(SPACING)
        .padding(PADDING)
        .with_flex(left, 1f32)
        .with_flex(middle, 2f32)
        .with_flex(right, 1f32);

    ctx.alloc(root)
}
