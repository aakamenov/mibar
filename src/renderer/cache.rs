// Cached rendering based on: https://rxi.github.io/cached_software_rendering.html

use std::{mem, hash::{Hash, Hasher}};

use ahash::AHasher;
use tiny_skia::{Color, PixmapMut};

use crate::geometry::{Rect, Point, Size};
use super::renderer::{Renderer, Quad, Circle, BorderRadius, Background};

const CELL_SIZE: f32 = 50f32;

pub struct CachedRenderer {
    renderer: Option<Renderer>,
    screen_size: Size,
    cells: Vec<u64>,
    cells_prev: Vec<u64>,
    dirty_regions: Vec<Rect>,
    commands: Vec<Command>
}

#[derive(Debug)]
pub enum Command {
    Draw(Primitive),
    Clip(Rect),
    PopClip
}

#[derive(Debug)]
pub enum Primitive {
    Quad(Quad),
    Circle(Circle)
}

impl CachedRenderer {
    #[inline]
    pub(crate) fn new() -> Self {
        Self {
            renderer: Some(Renderer::new()),
            screen_size: Size::ZERO,
            cells: Vec::new(),
            cells_prev: Vec::new(),
            dirty_regions: Vec::new(),
            commands: Vec::with_capacity(64)
        }
    }

    #[inline]
    pub fn fill_quad(&mut self, quad: Quad) {
        self.commands.push(
            Command::Draw(Primitive::Quad(quad))
        );
    }

    #[inline]
    pub fn fill_circle(&mut self, circle: Circle) {
        self.commands.push(
            Command::Draw(Primitive::Circle(circle))
        );
    }

    #[inline]
    pub fn push_clip(&mut self, clip: Rect) {
        self.commands.push(Command::Clip(clip));
    }

    #[inline]
    pub fn pop_clip(&mut self) {
        self.commands.push(Command::PopClip);
    }

    pub(crate) fn resize(&mut self, size: Size) {
        self.screen_size = size;

        let width = (size.width / CELL_SIZE).ceil() + 1f32;
        let height = (size.height / CELL_SIZE).ceil() + 1f32;
        let rect_buf_size = (size.width * size.height / 2f32).ceil() as usize;
        let size = (width * height) as usize;

        if size > self.cells.capacity() {
            let additional = size - self.cells.capacity();
            self.cells.reserve_exact(additional);
            self.cells_prev.reserve_exact(additional);

            let additional = size.saturating_sub(rect_buf_size)
                .saturating_sub(self.dirty_regions.capacity());

            self.dirty_regions.reserve_exact(additional);
        } else {
            self.cells.truncate(size);
            self.cells_prev.truncate(size);
            self.dirty_regions.truncate(rect_buf_size);
        };

        assert!(self.cells.capacity() >= size);
        assert!(self.cells_prev.capacity() >= size);

        unsafe {
            self.cells.set_len(size);
            self.cells_prev.set_len(size);
        }

        // Invalidate cache
        self.cells_prev.fill(0);
    }

    pub(crate) fn render(&mut self, pixmap: &mut PixmapMut) -> Vec<Rect> {
        let mut commands = mem::take(&mut self.commands);

        let mut screen = Rect::from_size(self.screen_size);
        let mut clip_stack: Vec<Rect> = Vec::with_capacity(8);

        for command in &commands {
            let rect = match command {
                Command::Draw(primitive) => match primitive {
                    Primitive::Quad(quad) => quad.rect,
                    Primitive::Circle(circle) => circle.bounds()
                },
                Command::Clip(rect) => {
                    let rect = *rect;
                    clip_stack.push(rect);
                    screen = rect;

                    rect
                },
                Command::PopClip => {
                    clip_stack.pop()
                        .expect("Pop clip commands must be preceeded by a push clip command.");

                    screen = clip_stack.last()
                        .copied()
                        .unwrap_or(Rect::from_size(self.screen_size));

                    continue;
                }
            };

            let Some(intersection) = rect.intersect(screen) else {
                continue;
            };

            let hash = hash_command(command);
            self.update_overlapping(intersection, hash);
        }

        let max_x = self.screen_size.width / CELL_SIZE + 1f32;
        let max_y = self.screen_size.height / CELL_SIZE + 1f32;

        for y in 0..max_y as u32 {
            for x in 0..max_x as u32 {
                let index = self.cell_index(x, y);

                if self.cells[index] != self.cells_prev[index] {
                    self.merge_and_push_rect(
                        Rect::new(x as f32, y as f32, 1f32, 1f32)
                    );
                }

                self.cells_prev[index] = 0;
            }
        }

        let screen = Rect::from_size(self.screen_size);
        let mut i = 0;

        while i < self.dirty_regions.len() {
            let rect = &mut self.dirty_regions[i];
            rect.x *= CELL_SIZE;
            rect.y *= CELL_SIZE;
            rect.width *= CELL_SIZE;
            rect.height *= CELL_SIZE;

            match rect.intersect(screen) {
                Some(intersection) => {
                    *rect = intersection;
                    i += 1;
                },
                None => { self.dirty_regions.swap_remove(i); }
            } 
        }

        self.redraw_dirty_regions(pixmap, &commands, clip_stack);

        // Assign back the buffer in order to reuse the memory.
        commands.clear();
        self.commands = commands;

        mem::swap(&mut self.cells, &mut self.cells_prev);
        
        // TODO: Re-write so that it's possible get a reference
        // to the damaged regions instead of wasting the memory each time.
        mem::take(&mut self.dirty_regions)
    }

    fn redraw_dirty_regions(
        &mut self,
        pixmap: &mut PixmapMut,
        commands: &[Command],
        mut clip_stack: Vec<Rect>
    ) {
        let mut pass = self.renderer.take().unwrap().begin(pixmap);

        for region in &self.dirty_regions {
            let mut clip = *region;
            clip_stack.clear();
            
            for command in commands {
                match command {
                    Command::Draw(primitive) => match primitive {
                        Primitive::Quad(quad) =>
                            if let Some(rect) = clip.intersect(quad.rect) {
                                if rect != quad.rect {
                                    pass.set_clip(rect);
                                    pass.draw_quad(quad.clone());
                                    pass.set_clip(clip);
                                } else {
                                    pass.draw_quad(quad.clone());
                                }
                            }
                        Primitive::Circle(circle) => {
                            let bounds = circle.bounds();
                            if let Some(rect) = clip.intersect(bounds) {
                                if rect != bounds {
                                    pass.set_clip(rect);
                                    pass.draw_circle(circle.clone());
                                    pass.set_clip(clip);
                                } else {
                                    pass.draw_circle(circle.clone());
                                }
                            }
                        }
                    },
                    Command::Clip(rect) => {
                        let rect = rect.intersect(*region).unwrap_or_default();
                        clip_stack.push(rect);
                        clip = rect;
                        pass.set_clip(clip);
                    },
                    Command::PopClip => {
                        clip_stack.pop();
                        clip = clip_stack.last().copied().unwrap_or(*region);
                        pass.set_clip(clip);
                    }
                }
            }
        }

        self.renderer = Some(pass.end());
    }

    fn update_overlapping(&mut self, rect: Rect, hash: u64) {
        let mut hasher = AHasher::default();

        let x = (rect.x / CELL_SIZE) as u32;
        let y = (rect.y / CELL_SIZE) as u32;
        let width = ((rect.x + rect.width) / CELL_SIZE) as u32;
        let height = ((rect.y + rect.height) / CELL_SIZE) as u32;

        for y in y..=height {
            for x in x..=width {
                let index = self.cell_index(x, y);

                hasher.write_u64(hash);
                hasher.write_u64(self.cells[index]);
                self.cells[index] = hasher.finish();
            }
        }
    }

    #[inline]
    fn merge_and_push_rect(&mut self, cell_rect: Rect) {
        let mut index = self.dirty_regions.len().saturating_sub(1);

        while index > 0 {
            let rect = &mut self.dirty_regions[index];
            if rect.overlaps(cell_rect) {
                // Merge rects
                let x = rect.x.min(cell_rect.x);
                let y = rect.y.min(cell_rect.y);
                let width = (rect.x + rect.width).max(cell_rect.x + cell_rect.width);
                let height = (rect.y + rect.height).max(cell_rect.y + cell_rect.height);

                rect.x = x;
                rect.y = y;
                rect.width = width - x;
                rect.height = height - y;

                return;
            }
            
            index -= 1;
        }

        self.dirty_regions.push(cell_rect);
    }

    #[inline]
    fn cell_index(&self, x: u32, y: u32) -> usize  {
        (x + y * (self.screen_size.width / CELL_SIZE) as u32) as usize
    }
}

impl Hash for Background {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            Background::Color(color) => hash_color(*color, state),
            Background::LinearGradient(gradient) => {
                let dbg = format!("{:?}", gradient);
                dbg.hash(state);
            }
        }
    }
}

impl Hash for BorderRadius {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0[0].to_bits().hash(state);
        self.0[1].to_bits().hash(state);
        self.0[2].to_bits().hash(state);
        self.0[3].to_bits().hash(state);
    }
}

impl Hash for Quad {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        hash_rect(self.rect, state);
        self.background.hash(state);
        self.border_radius.hash(state);
        self.border_width.to_bits().hash(state);
        self.border_color.hash(state);
    }
}

impl Hash for Circle {
    fn hash<H: Hasher>(&self, state: &mut H) {
        hash_point(self.pos, state);
        self.radius.to_bits().hash(state);
        self.background.hash(state);
        self.border_width.to_bits().hash(state);
        self.border_color.hash(state);
    }
}

#[inline(always)]
fn hash_command(command: &Command) -> u64 {
    let mut hasher = AHasher::default();

    match command {
        Command::Draw(primitive) => match primitive {
            Primitive::Quad(quad) => quad.hash(&mut hasher),
            Primitive::Circle(circle) => circle.hash(&mut hasher),
        },
        Command::Clip(rect) => hash_rect(*rect, &mut hasher),
        Command::PopClip => unreachable!()
    }

    hasher.finish()
}

#[inline(always)]
fn hash_color(color: Color, hasher: &mut impl Hasher) {
    color.red().to_bits().hash(hasher);
    color.green().to_bits().hash(hasher);
    color.blue().to_bits().hash(hasher);
    color.alpha().to_bits().hash(hasher);
}

#[inline(always)]
fn hash_rect(rect: Rect, hasher: &mut impl Hasher) {
    rect.x.to_bits().hash(hasher);
    rect.y.to_bits().hash(hasher);
    rect.width.to_bits().hash(hasher);
    rect.height.to_bits().hash(hasher);
}

#[inline(always)]
fn hash_point(point: Point, hasher: &mut impl Hasher) {
    point.x.to_bits().hash(hasher);
    point.y.to_bits().hash(hasher);
}
