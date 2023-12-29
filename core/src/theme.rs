use std::{ptr, cell::Cell};

use cosmic_text::{Family, Stretch, Style, Weight};
use crate::{widget::{text, button}, Color};

const COLOR_OVERWRITE_CAP: usize = 16;

#[derive(Debug)]
pub struct Theme {
    pub font: Font,
    pub font_size: f32,
    pub button: button::StyleFn,
    text: text::StyleFn,
    text_colors: (Cell<ColorPtr>, [Color; COLOR_OVERWRITE_CAP])
}

#[derive(Clone, Copy, PartialEq, Hash, Debug)]
pub struct Font {
    pub family: Family<'static>,
    pub stretch: Stretch,
    pub style: Style,
    pub weight: Weight
}

#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Debug)]
struct ColorPtr(*mut Color);

impl Theme {
    #[inline]
    pub fn new(
        font: Font,
        font_size: f32,
        text: text::StyleFn,
        button: button::StyleFn
    ) -> Self {
        Self {
            font,
            font_size,
            text,
            button,
            text_colors: (
                Cell::new(ColorPtr(ptr::null_mut())),
                [Color::WHITE; COLOR_OVERWRITE_CAP]
            )
        }
    }

    pub fn text_color(&self) -> Color {
        let ptr = self.text_colors.0.get().0;

        if !ptr.is_null() {
            unsafe { *ptr }
        } else {
            (self.text)()
        }
    }

    pub fn push_text_color(&self, color: Color) {
        let mut ptr = self.text_colors.0.get().0;
        let array = &self.text_colors.1[0] as *const _ as *mut Color;

        if ptr.is_null() {
            ptr = array;
            unsafe { *ptr = color; }
        } else {
            let end = unsafe { array.add(COLOR_OVERWRITE_CAP - 1) };
            if ptr == end {
                panic!("Color override array capacity exceeded. MAX: 16");
            }

            unsafe {
                ptr = ptr.add(1);
                *ptr = color;
            } 
        }

        self.text_colors.0.set(ColorPtr(ptr));
    }

    pub fn pop_text_color(&self) {
        let mut ptr = self.text_colors.0.get().0;
        let array = &self.text_colors.1[0] as *const _ as *mut Color;

        if ptr.is_null() {
            return;
        }

        if ptr == array {
            ptr = ptr::null_mut();
        } else {
            ptr = unsafe { ptr.sub(1) };
        }

        self.text_colors.0.set(ColorPtr(ptr));
    }
}

impl Default for Font {
    #[inline]
    fn default() -> Self {
        Self {
            family: Family::SansSerif,
            stretch: Stretch::Normal,
            style: Style::Normal,
            weight: Weight::NORMAL
        }
    }
}

impl Clone for Theme {
    #[inline]
    fn clone(&self) -> Self {
        Self::new(self.font.clone(), self.font_size, self.text, self.button)
    }
}

unsafe impl Sync for ColorPtr { }
unsafe impl Send for ColorPtr { }

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{QuadStyle, widget::button};

    fn theme() -> Theme {
        Theme::new(
            Font::default(),
            16f32,
            || Color::TRANSPARENT,
            |_| button::Style {
                quad: QuadStyle::solid_background(Color::RED),
                text_color: None
            }
        )
    } 

    #[test]
    fn read_write_colors() {
        let theme = theme();
        theme.push_text_color(Color::RED);
        theme.push_text_color(Color::GREEN);
        theme.push_text_color(Color::BLUE);

        assert_eq!(theme.text_color(), Color::BLUE);
        theme.pop_text_color();

        assert_eq!(theme.text_color(), Color::GREEN);
        theme.pop_text_color();

        assert_eq!(theme.text_color(), Color::RED);
        theme.pop_text_color();

        assert_eq!(theme.text_color(), Color::TRANSPARENT);
    }

    #[test]
    #[should_panic]
    fn exceed_color_stack_capacity() {
        let theme = theme();

        for _ in 0..=COLOR_OVERWRITE_CAP {
            theme.push_text_color(Color::BLACK);
        }
    }
}
