use mibar::{
    tokio,
    modules::{
        workspaces::{self, Workspaces},
        date_time::DateTime,
        battery::{self, Battery},
        cpu::Cpu,
        ram::Ram,
        volume::{pulseaudio, PulseAudioVolume},
        sys_info
    },
    widget::{
        flex::{Flex, FlexBuilder},
        Element, Padding, Alignment
    },
    window::bar::{Bar, Location},
    Theme, Font, Family, Color, QuadStyle, run
};

// Color palette: https://coolors.co/232f2e-293635-aca695-d9ddde-ff8000-70d900-ff4c57-00dbd7-ff64a2

const PADDING: Padding = Padding::new(2f32, 6f32, 2f32, 6f32);
const SPACING: f32 = 10f32;

const PRIMARY_RED: Color = Color::rgb(255, 76, 87);
const PRIMARY_GREEN: Color = Color::rgb(112, 217, 0);
//const PRIMARY_BLUE: Color = Color::rgb(0, 219, 215);
const PRIMARY_ORANGE: Color = Color::rgb(255, 128, 0);
//const PRIMARY_PINK: Color = Color::rgb(255, 100, 162);

const BASE: Color = Color::rgb(35, 47, 46);
//const BACKGROUND: Color = Color::rgb(41, 54, 53);
const TEXT: Color = Color::rgb(217, 221, 222);

const PRIMARY: Color = PRIMARY_GREEN;
const OUTLINE: Color = Color::rgb(172, 166, 149);

fn main() {
    let mut builder = tokio::runtime::Builder::new_multi_thread();
    builder.enable_all();

    let window = Bar::new(40, Location::Top);

    run(builder, window, build(), theme(), |_| sys_info::init());
}

fn theme() -> Theme {
    Theme {
        font: Font {
            family: Family::Name("SauceCodePro Nerd Font"),
            ..Font::default()
        },
        font_size: 16f32,
        text: || TEXT,
        button: |_| QuadStyle::solid_background(Color::TRANSPARENT)
            .rounded(4f32)
            .with_border(1f32, OUTLINE)
    }
}

fn build() -> impl Element {
    let create = |builder: &mut FlexBuilder| {
        let left = Flex::row(|builder| {
            builder.add_non_flex(Workspaces::new(workspaces_style));
            builder.add_non_flex(DateTime::new());
        })
        .spacing(SPACING);

        builder.add_flex(left, 1f32);
        
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
        .style(|| QuadStyle::solid_background(BASE))
}

fn battery_style(capacity: u8) -> battery::Style {
    let (background, text) = if capacity >= 80 {
        (PRIMARY_GREEN, BASE)
    } else if capacity > 20 {
        (PRIMARY_ORANGE, TEXT)
    } else {
        (PRIMARY_RED, TEXT)
    };

    battery::Style {
        body: OUTLINE,
        background: background.into(),
        text
    }
}

fn workspaces_style() -> workspaces::Style {
    workspaces::Style {
        active: PRIMARY,
        empty: OUTLINE,
        text_color: TEXT,
        selected_text_color: BASE
    }
}

fn format_audio(state: pulseaudio::State) -> String {
    if state.is_muted {
        return "󰝟 ".into();
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
