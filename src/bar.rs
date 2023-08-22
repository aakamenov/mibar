use crate::{
    modules::{
        Workspaces,
        DateTime
    },
    widget::{
        music::Music,
        cpu::Cpu,
        ram::Ram,
        flex::{Flex, FlexBuilder, Alignment},
        Element
    }
};

const PADDING: f32 = 6f32;
const SPACING: f32 = 10f32;

pub fn build() -> impl Element {
    let create = |builder: &mut FlexBuilder| {
        let left = Flex::row(|builder| {
            builder.add_non_flex(Workspaces);
            builder.add_non_flex(DateTime);
        })
        .spacing(SPACING);

        builder.add_flex(left, 1f32);

        let middle = Flex::row(|builder| {
            builder.add_non_flex(Music);
        })
        .spacing(SPACING);

        builder.add_flex(middle, 2f32);

        let right = Flex::row(|builder| {
            builder.add_non_flex(Cpu);
            builder.add_non_flex(Ram);
        })
        .main_alignment(Alignment::End)
        .spacing(SPACING);

        builder.add_flex(right, 1f32);
    };

    Flex::row(create)
        .spacing(SPACING)
        .padding(PADDING)
}
