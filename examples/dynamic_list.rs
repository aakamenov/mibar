use mibar_core::{
    tokio,
    window::{
        side_panel::{SidePanel, Location},
        WindowDimensions
    },
    widget::{
        ButtonState, Flex, Text, Button, Length, Element,
        Alignment, Container, container, button
    },
    Context, Id, Theme, Color, QuadStyle, Font, ReactiveList, UniqueKey, run
};

const BLUE: Color = Color::rgb(13, 110, 253);
const RED: Color = Color::rgb(220, 53, 69);
const GREY: Color = Color::rgb(173, 181, 189);
const BORDER_ROUNDING: f32 = 4f32;
const PADDING: f32 = 12f32;
const SPACING: f32 = 8f32;

#[derive(Hash, PartialEq, Eq)]
struct Item {
    id: usize,
    selected: bool
}

impl UniqueKey for Item {
    type Key = usize;

    #[inline]
    fn key(&self) -> &Self::Key {
        &self.id
    }
}

fn main() {
    let builder = tokio::runtime::Builder::new_current_thread();
    let window = SidePanel::new(WindowDimensions::Fixed((400, 600)), Location::TopRight);

    let theme = Theme::new(
        Font::default(),
        16f32,
        || Color::BLACK,
        button_primary
    );

    run(builder, window, build, theme);
}

fn build(ctx: &mut Context) -> Id {
    let items = ReactiveList::<Item>::new();

    let item_controls = item_controls(ctx, items.clone());

    let items_column = Flex::column()
        .padding(PADDING)
        .spacing(SPACING)
        .main_alignment(Alignment::Start)
        .bind(ctx, &items.clone(), move |builder, item| {
            let items = items.clone();
            let id = item.id;

            let container = Container::new(Text::new(format!("Item {}", item.id)))
                .padding(4f32)
                .horizontal_alignment(Alignment::Start)
                .width(Length::Expand)
                .style(container_unselected)
                .make(builder.ctx);

            let delete_button = {
                let items = items.clone();

                Button::new("X", move |ctx| {
                    items.mutate(ctx, |items| {
                        let index = items.iter().position(|x| x.id == id).unwrap();
                        items.remove(index);
                    });
                })
                .width(Length::Expand)
                .style(button_danger)
            };

            let text = Text::new("Select").make(builder.ctx);
            let select_button = Button::with_child(text, move |ctx| {
                let index = items.as_slice().iter().position(|x| x.id == id).unwrap();

                let is_selected = &mut items.as_mut_slice()[index].selected;
                let selected = *is_selected;
                *is_selected = !selected;

                if selected {
                    container.set_style(ctx, container_unselected);
                    text.set_text(ctx, "Select");
                } else {
                    container.set_style(ctx, container_selected);
                    text.set_text(ctx, "Deselect");
                }
            })
            .width(Length::Expand);

            builder.non_flex(Flex::row()
                .spacing(4f32)
                .build((
                    (container, 2f32),
                    (delete_button, 1f32),
                    (select_button, 1f32)
                ))
            )
        });

    Flex::column()
        .style(|| QuadStyle::solid_background(Color::WHITE))
        .build((
            (items_column, 5f32),
            (item_controls, 1f32)
        ))
        .make(ctx)
        .into()
}

fn item_controls(
    ctx: &mut Context,
    items: ReactiveList<Item>
) -> impl Element {
    let counter = ctx.tree.set_context(1usize);

    Flex::column()
        .padding(PADDING)
        .spacing(SPACING)
        .build((
            ({
                let items = items.clone();

                Button::new("Remove selected", move |ctx| {
                    items.mutate(ctx, |items| items.retain(|x| !x.selected));
                })
                .style(button_danger)
                .width(Length::Expand)},
                1f32
            ),
            (
                Button::new("+", move |ctx| {
                    let count = &mut ctx.tree[counter];
                    let id = *count;
                    *count += 1;

                    items.mutate(ctx, |items| items.push(Item { id, selected: false }))
                })
                .width(Length::Expand),
                1f32
            )
        ))
}

#[inline]
fn container_unselected() -> container::Style {
    container::Style {
        quad: QuadStyle::bordered(GREY, 1f32).rounded(BORDER_ROUNDING),
        text_color: None
    }
}

#[inline]
fn container_selected() -> container::Style {
    container::Style {
        quad: QuadStyle::solid_background(GREY).rounded(BORDER_ROUNDING),
        text_color: Some(Color::TRANSPARENT)
    }
}

fn button_primary(state: button::ButtonState) -> button::Style {
    button_style(state, BLUE)
}

fn button_danger(state: button::ButtonState) -> button::Style {
    button_style(state, RED)
}

fn button_style(state: button::ButtonState, color: Color) -> button::Style {
    let (bg, text_color) = match state {
        ButtonState::Normal => (Color::TRANSPARENT, None),
        ButtonState::Hovered | ButtonState::Active => (color, Some(Color::WHITE)),
    };

    let quad = QuadStyle::solid_background(bg)
        .rounded(BORDER_ROUNDING)
        .with_border(1f32, color);

    button::Style { quad, text_color }
}
