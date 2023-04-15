use crate::{
    geometry::{Size, Rect},
    positioner::Positioner,
    ui::DrawCtx
};
use super::{
    size_constraints::SizeConstraints,
    Widget
};

pub struct Flex {
    flex: Vec<(Box<dyn Widget>, f32)>,
    non_flex: Vec<Box<dyn Widget>>,
    rects: Vec<Rect>,
    axis: Axis,
    alignment: Alignment,
    spacing: f32,
    padding: f32
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Alignment {
    Start,
    Center,
    End
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Axis {
    Horizontal,
    Vertical
}

impl Flex {
    #[inline]
    pub fn row() -> Self {
        Self::new(Axis::Horizontal)
    }

    #[inline]
    pub fn column() -> Self {
        Self::new(Axis::Vertical)
    }

    #[inline]
    pub fn padding(mut self, padding: f32) -> Self {
        self.padding = padding;

        self
    }

    #[inline]
    pub fn spacing(mut self, spacing: f32) -> Self {
        self.spacing = spacing;

        self
    }

    #[inline]
    pub fn alignment(mut self, alignment: Alignment) -> Self {
        self.alignment = alignment;

        self
    }

    #[inline]
    pub fn with_non_flex(
        mut self,
        child: impl Widget + 'static
    ) -> Self {
        self.non_flex.push(Box::new(child));

        self
    }

    #[inline]
    pub fn with_flex(
        mut self,
        child: impl Widget + 'static,
        flex: f32
    ) -> Self {
        self.flex.push((Box::new(child), flex));

        self
    }

    #[inline]
    fn new(axis: Axis) -> Self {
        Self {
            flex: Vec::new(),
            non_flex: Vec::new(),
            rects: Vec::new(),
            axis,
            alignment: Alignment::Center,
            spacing: 0f32,
            padding: 0f32
        }
    }
}

impl Widget for Flex {
    // Simplified version of the Flutter flex layout algorithm:
    // https://api.flutter.dev/flutter/widgets/Flex-class.html
    fn layout(&mut self, bounds: SizeConstraints) -> Size {
        let total_len = self.flex.len() + self.non_flex.len();

        self.rects.clear();
        self.rects.reserve(total_len);

        let bounds = bounds.shrink(
            Size::new(self.padding, self.padding)
        );
        let spacing = self.spacing * total_len.saturating_sub(1) as f32;

        let max_cross = self.axis.cross(bounds.max);
        let mut cross = self.axis.cross(bounds.min);

        let mut available = self.axis.main(bounds.max) - spacing;

        for child in &mut self.non_flex {
            let (width, height) = self.axis.main_and_cross(available, max_cross);
            let widget_bounds = SizeConstraints::new(
                Size::ZERO,
                Size::new(width, height)
            );

            let size = child.layout(widget_bounds);
            available -= self.axis.main(size);
            cross = cross.max(self.axis.cross(size));

            self.rects.push(Rect {
                x: 0f32,
                y: 0f32,
                width: size.width,
                height: size.height
            });
        }

        let available = available.max(0f32);
        let total_flex: f32 = self.flex.iter().map(|x| x.1).sum();

        for child in &mut self.flex {
            let max_main = available * child.1 / total_flex;
            let min_main = if max_main.is_infinite() {
                0.0
            } else {
                max_main
            };

            let (min_width, min_height) = self.axis.main_and_cross(
                min_main,
                self.axis.cross(bounds.min)
            );

            let (max_width, max_height) = self.axis.main_and_cross(
                max_main,
                max_cross
            );

            let widget_bounds = SizeConstraints::new(
                Size::new(min_width, min_height),
                Size::new(max_width, max_height)
            );

            let size = child.0.layout(widget_bounds);
            cross = cross.max(self.axis.cross(size));

            self.rects.push(Rect {
                x: 0f32,
                y: 0f32,
                width: size.width,
                height: size.height
            });
        }

        let mut main = self.padding;

        for (i, rect) in self.rects.iter_mut().enumerate() {
            if i > 0 {
                main += self.spacing;
            }

            let (x, y) = self.axis.main_and_cross(main, self.padding);
            rect.x = x;
            rect.y = y; 

            match self.axis {
                Axis::Horizontal => match self.alignment {
                    Alignment::Start => {}
                    Alignment::Center => {
                        rect.y += (cross - rect.height) / 2.0;
                    }
                    Alignment::End => {
                        rect.y += cross - rect.height;
                    }
                },
                Axis::Vertical => match self.alignment {
                    Alignment::Start => {}
                    Alignment::Center => {
                        rect.x += (cross - rect.width) / 2.0;
                    }
                    Alignment::End => {
                        rect.x += cross - rect.width;
                    }
                }
            }

            main += self.axis.main(rect.size());
        }

        let (width, height) = self.axis.main_and_cross(main - self.padding, cross);
        
        bounds.constrain(Size::new(width, height))
    }

    fn draw(&mut self, ctx: &mut DrawCtx, positioner: Positioner) {
        for (i, child) in self.non_flex.iter_mut().chain(
            self.flex.iter_mut().map(|x| &mut x.0)
        ).enumerate() {
            let positioner = positioner.next(self.rects[i]);

            child.draw(ctx, positioner);
        }
    }
}

impl Axis {
    #[inline]
    pub fn main(&self, size: Size) -> f32 {
        match self {
            Self::Horizontal => size.width,
            Self::Vertical => size.height
        }
    }

    #[inline]
    pub fn cross(&self, size: Size) -> f32 {
        match self {
            Self::Horizontal => size.height,
            Self::Vertical => size.width
        }
    }

    #[inline]
    pub fn main_and_cross(&self, main: f32, cross: f32) -> (f32, f32) {
        match self {
            Self::Horizontal => (main, cross),
            Self::Vertical => (cross, main)
        }
    }
}

