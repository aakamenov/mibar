use std::{time::Duration, process::Command};

use mibar::{
    tokio,
    modules::{
        workspaces::{self, Workspaces},
        date_time::DateTime,
        battery::{self, Battery},
        cpu::Cpu,
        ram::Ram,
        volume::{pulseaudio, PulseAudioVolume},
        keyboard_layout::KeyboardLayout
    },
    widget::{
        button::{self, ButtonState},
        Element, Padding, Alignment,
        Button, Text, Flex, FlexBuilder, State, StateHandle
    },
    window::{
        bar::{self, Bar},
        side_panel::{self, SidePanel},
        WindowId
    },
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

const KEYBOARD_DEVICE: &str = "ducky-ducky-one-3-sf-rgb-1";
const BATTERY_DEVICE: &str = "BAT0";

const BAR_SIZE: u32 = 40;

#[derive(Debug)]
struct BarState {
    power_menu: Option<WindowId>
}

fn main() {
    let mut builder = tokio::runtime::Builder::new_multi_thread();
    builder.worker_threads(4);
    builder.enable_all();

    let window = Bar::new(BAR_SIZE, bar::Location::Top);

    run(builder, window, build, theme());
}

fn font() -> Font {
    Font {
        family: Family::Name("SauceCodePro Nerd Font"),
        ..Font::default()
    }
}

fn font_mono() -> Font {
    Font {
        family: Family::Name("SauceCodePro Nerd Font Mono"),
        ..Font::default()
    }
}

fn theme() -> Theme {
    Theme::new(
        font(),
        16f32,
        || TEXT,
        |state| {
            let (bg, text_color) = match state {
                ButtonState::Normal => (Color::TRANSPARENT, None),
                ButtonState::Hovered | ButtonState::Active => (OUTLINE, Some(BASE)),
            };

            let quad = QuadStyle::solid_background(bg)
                .rounded(4f32)
                .with_border(1f32, OUTLINE);

            button::Style { quad, text_color }
        }
    )
}

fn build() -> impl Element {
    let state = BarState {
        power_menu: None
    };

    State::new(state, |handle| {
        let create = |builder: &mut FlexBuilder| {
            let left = Flex::row(|builder| {
                builder.add_non_flex(Workspaces::new(workspaces_style));
                builder.add_non_flex(DateTime::new());
            })
            .spacing(SPACING);

            builder.add_flex(left, 1f32);
            
            let right = Flex::row(|builder| {
                builder.add_non_flex(KeyboardLayout::new(KEYBOARD_DEVICE));
                builder.add_non_flex(PulseAudioVolume::new(format_audio));
                builder.add_non_flex(Battery::new(BATTERY_DEVICE, Duration::from_secs(30), battery_style));
                builder.add_non_flex(Cpu::new());
                builder.add_non_flex(Ram::new());
                builder.add_non_flex(boot_menu_button(handle));
            })
            .main_alignment(Alignment::End)
            .spacing(SPACING);

            builder.add_flex(right, 1f32);
        };

        Flex::row(create)
            .spacing(SPACING)
            .padding(PADDING)
            .style(|| QuadStyle::solid_background(BASE))
    })
}

fn boot_menu_button(mut state: StateHandle<BarState>) -> Button<Text> {
    let text = Text::new("⏻")
        .text_size(22f32)
        // Use the monospaced font because the icon gets cut otherwise.
        // https://github.com/pop-os/cosmic-text/issues/182
        .font(font_mono());

    let size = BAR_SIZE as f32 - PADDING.vertical();
    Button::with_child(text, move |ctx, _| {
        match state.power_menu {
            Some(window_id) if window_id.is_alive() => ctx.close_window(window_id),
            _ => {
                let panel = SidePanel::new(
                    (155, 55),
                    side_panel::Location::TopRight
                );

                let window_id = ctx.open_window(panel, boot_menu_panel);
                state.power_menu = Some(window_id);
            }
        }
    })
    .width(size)
    .height(size)
    .style(|state| {
        let (bg, text_color) = match state {
            ButtonState::Normal => (Color::TRANSPARENT, PRIMARY_RED),
            ButtonState::Hovered | ButtonState::Active => (PRIMARY_RED, BASE)
        };

        button::Style {
            quad: QuadStyle::solid_background(bg)
                .with_border(2f32, PRIMARY_RED),
            text_color: Some(text_color)
        }
    })
}

fn boot_menu_panel() -> impl Element {
    fn button_style(state: ButtonState, color: Color) -> button::Style {
        let (bg, text_color) = match state {
            ButtonState::Normal => (Color::TRANSPARENT, color),
            ButtonState::Hovered | ButtonState::Active => (color, BASE)
        };

        button::Style {
            quad: QuadStyle::solid_background(bg),
            text_color: Some(text_color)
        }
    }

    Flex::row(|builder| {
        const ICON_SIZE: f32 = 24f32;

        builder.add_non_flex({
            let col = Flex::column(|builder| {
                builder.add_non_flex(Text::new("⏻").font(font_mono()).text_size(ICON_SIZE));
                builder.add_non_flex(Text::new("Shutdown").font(font_mono()));
            });

            Button::with_child(col, |_, _| {
                if let Err(err) = Command::new("shutdown").arg("-h").arg("now").spawn() {
                    eprintln!("Failed to execute shutdown command: {err}");
                }
            })
            .padding(0f32)
            .style(|state| button_style(state, PRIMARY_RED))
        });

        builder.add_non_flex({
            let col = Flex::column(|builder| {
                builder.add_non_flex(Text::new("󰜉").font(font_mono()).text_size(ICON_SIZE));
                builder.add_non_flex(Text::new("Reboot").font(font_mono()));
            });

            Button::with_child(col, |_, _| {
                if let Err(err) = Command::new("reboot").spawn() {
                    eprintln!("Failed to execute reboot command: {err}");
                }
            })
            .padding(0f32)
            .style(|state| button_style(state, PRIMARY_ORANGE))
        });
    })
    .spacing(SPACING)
    .padding(PADDING)
    .style(|| QuadStyle::solid_background(BASE).with_border(1f32, OUTLINE))
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
