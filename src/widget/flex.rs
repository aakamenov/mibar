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
    children: Vec<(Box<dyn Widget>, f32)>,
    rects: Vec<Rect>,
    axis: Axis,
    main_alignment: Alignment,
    cross_alignment: Alignment,
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
    pub fn cross_alignment(mut self, alignment: Alignment) -> Self {
        self.cross_alignment = alignment;

        self
    }

    #[inline]
    pub fn main_alignment(mut self, alignment: Alignment) -> Self {
        self.main_alignment = alignment;

        self
    }

    #[inline]
    pub fn with_non_flex(
        self,
        child: impl Widget + 'static
    ) -> Self {
        self.with_flex(child, 0f32)
    }

    #[inline]
    pub fn with_flex(
        mut self,
        child: impl Widget + 'static,
        flex: f32
    ) -> Self {
        self.children.push((Box::new(child), flex));

        self
    }

    #[inline]
    fn new(axis: Axis) -> Self {
        Self {
            children: Vec::new(),
            rects: Vec::new(),
            axis,
            main_alignment: Alignment::Start,
            cross_alignment: Alignment::Center,
            spacing: 0f32,
            padding: 0f32
        }
    }
}

impl Widget for Flex {
    // Simplified version of the Flutter flex layout algorithm:
    // https://api.flutter.dev/flutter/widgets/Flex-class.html
    fn layout(&mut self, bounds: SizeConstraints) -> Size {
        let total_len = self.children.len();

        self.rects.clear();
        self.rects.reserve(total_len);

        unsafe {
            self.rects.set_len(total_len);
        }

        let bounds = bounds.shrink(
            Size::new(self.padding * 2f32, self.padding * 2f32)
        );
        let spacing = self.spacing * total_len.saturating_sub(1) as f32;

        let max_cross = self.axis.cross(bounds.max);
        let mut cross = self.axis.cross(bounds.min);
        let mut total_main = 0f32;

        let mut available = self.axis.main(bounds.max) - spacing;
        let mut total_flex = 0f32;

        // Layout non-flex children first i.e those with flex factor == 0
        for (i, (child, flex)) in self.children.iter_mut().enumerate() {
            total_flex += *flex;

            if flex.abs() > 0f32 {
                continue;
            }

            let (width, height) = self.axis.main_and_cross(available, max_cross);
            let widget_bounds = SizeConstraints::new(
                Size::ZERO,
                Size::new(width, height)
            );

            let size = child.layout(widget_bounds);

            let main_cross = self.axis.main_and_cross_size(size);
            available -= main_cross.0;
            total_main += main_cross.0;
            cross = cross.max(main_cross.1);

            self.rects[i] = Rect {
                x: 0f32,
                y: 0f32,
                width: size.width,
                height: size.height
            };
        }

        if total_flex > 0f32 {
            let available = available.max(0f32);

            // Layout flex children i.e those with flex factor > 0
            for (i, (child, flex)) in self.children.iter_mut().enumerate() {
                if *flex <= 0f32 {
                    continue;
                }

                let max_main = available * *flex / total_flex;
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

                let size = child.layout(widget_bounds);

                let main_cross = self.axis.main_and_cross_size(size);
                total_main += main_cross.0;
                cross = cross.max(main_cross.1);

                self.rects[i] = Rect {
                    x: 0f32,
                    y: 0f32,
                    width: size.width,
                    height: size.height
                };
            }
        }

        let mut main = match self.main_alignment {
            Alignment::Start => self.padding,
            Alignment::Center => (self.axis.main(bounds.max) -
                spacing -
                total_main) /
                2f32,
            Alignment::End => self.axis.main(bounds.max) -
                spacing -
                total_main
        };

        // Position children
        for (i, rect) in self.rects.iter_mut().enumerate() {
            if i > 0 {
                main += self.spacing;
            }

            let (x, y) = self.axis.main_and_cross(main, self.padding);
            rect.x = x;
            rect.y = y;

            self.cross_alignment.align(rect, cross, self.axis.flip());

            main += self.axis.main(rect.size());
        }

        let (width, height) = self.axis.main_and_cross(main - self.padding, cross);
        
        bounds.constrain(Size::new(width, height))
    }

    fn draw(&mut self, ctx: &mut DrawCtx, positioner: Positioner) {
        for (i, (child, _)) in self.children.iter_mut().enumerate() {
            let positioner = positioner.next(self.rects[i]);

            child.draw(ctx, positioner);
        }
    }
}

impl Axis {
    #[inline]
    pub fn flip(&self) -> Self {
        match self {
            Self::Horizontal => Self::Vertical,
            Self::Vertical => Self::Horizontal
        }
    }

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
    pub fn main_and_cross_size(&self, size: Size) -> (f32, f32) {
        match self {
            Self::Horizontal => (size.width, size.height),
            Self::Vertical => (size.height, size.width)
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

impl Alignment {
    fn align(&self, rect: &mut Rect, space: f32, axis: Axis) {
        let (value, size) = match axis {
            Axis::Horizontal => (&mut rect.x, rect.width),
            Axis::Vertical => (&mut rect.y, rect.height)
        };

        match self {
            Self::Start => { }
            Self::Center => *value += (space - size) / 2f32,
            Self::End =>  *value += space - size
        }
    }
}
