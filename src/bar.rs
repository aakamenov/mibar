use cosmic_text::Family;

use crate::{
    modules::{
        workspaces::{self, Workspaces},
        date_time::DateTime,
        battery::{self, Battery},
        cpu::Cpu,
        ram::Ram,
        volume::{pulseaudio, PulseAudioVolume}
    },
    widget::{
        music::Music,
        flex::{self, Flex, FlexBuilder, Alignment},
        text,
        Element
    },
    theme::{Theme, Font},
    color::Color
};

const PADDING: f32 = 6f32;
const SPACING: f32 = 10f32;

const BASE: Color = Color::rgb(250, 244, 237);
//const SURFACE: Color = Color::rgb(255, 250, 243);
//const OVERLAY: Color = Color::rgb(242, 233, 222);
const MUTED: Color = Color::rgb(152, 147, 165);
const SUBTLE: Color = Color::rgb(121, 117, 147);
const TEXT: Color = Color::rgb(87, 82, 121);
const LOVE: Color = Color::rgb(180, 99, 122);
const GOLD: Color = Color::rgb(234, 157, 52);
//const ROSE: Color = Color::rgb(215, 130, 126);
//const PINE: Color = Color::rgb(40, 105, 131);
//const FOAM: Color = Color::rgb(86, 148, 159);
const IRIS: Color = Color::rgb(144, 122, 169);
//const HIGHLIGHT_LOW: Color = Color::rgb(244, 237, 232);
//const HIGHLIGHT_MEDIUM: Color = Color::rgb(223, 218, 217);
//const HIGHLIGHT_HIGH: Color = Color::rgb(206, 202, 205);

const LOVE_HOVER: Color = Color::rgb(186, 111, 132);
const LOVE_ACTIVE: Color = Color::rgb(191, 122, 142);

const MUTED_HOVER: Color = Color::rgb(159, 154, 171);
const MUTED_ACTIVE: Color = Color::rgb(166, 162, 177);


pub fn theme() -> Theme {
    Theme {
        font: Font {
            family: Family::Name("SauceCodePro Nerd Font"),
            ..Font::default()
        },
        font_size: 16f32,
        text: || text::Style { color: TEXT },
        flex: || { None }
    }
}

pub fn build() -> impl Element {
    let create = |builder: &mut FlexBuilder| {
        let left = Flex::row(|builder| {
            builder.add_non_flex(Workspaces::new(workspaces_style));
            builder.add_non_flex(DateTime::new());
        })
        .spacing(SPACING);

        builder.add_flex(left, 1f32);

        let middle = Flex::row(|builder| {
            builder.add_non_flex(Music);
        })
        .spacing(SPACING);

        builder.add_flex(middle, 2f32);

        let right = Flex::row(|builder| {
            builder.add_non_flex(PulseAudioVolume::new(format_audio));
            builder.add_non_flex(Battery::new(battery_style));
            builder.add_non_flex(Cpu::new());
            builder.add_non_flex(Ram::new());
        })
        .main_alignment(Alignment::End)
        .spacing(SPACING);

        builder.add_flex(right, 1f32);
    };

    Flex::row(create)
        .spacing(SPACING)
        .padding(PADDING)
        .style(|| Some(flex::Style::solid_background(BASE)))
}

fn battery_style(capacity: u8) -> battery::Style {
    let background = if capacity >= 80 {
        GOLD
    } else if capacity >= 20 {
        IRIS
    } else {
        LOVE
    };

    battery::Style {
        body: SUBTLE,
        background: background.into(),
        text: TEXT
    }
}

fn workspaces_style() -> workspaces::Style {
    workspaces::Style {
        active: workspaces::ButtonStyle {
            color: LOVE,
            hovered: LOVE_HOVER,
            active: LOVE_ACTIVE
        },
        inactive: workspaces::ButtonStyle {
            color: MUTED,
            hovered: MUTED_HOVER,
            active: MUTED_ACTIVE
        },
        text_color: TEXT,
        selected_text_color: BASE
    }
}

fn format_audio(state: pulseaudio::State) -> String {
    if state.is_muted {
        return "󰝟".into();
    }

    let icon = if state.volume >= 80 {
        "󰕾"
    } else if state.volume >= 20 {
        "󰖀"
    } else {
        "󰕿"
    };

    format!("{} {}", icon, state.volume)
}
