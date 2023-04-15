use crate::geometry::{Rect, Size};

#[derive(Debug)]
pub struct Positioner {
    pub bounds: Rect
}

impl Positioner {
    #[inline]
    pub fn new(size: Size) -> Self {
        Self {
            bounds: Rect {
                x: 0f32,
                y: 0f32,
                width: size.width,
                height: size.height
            } 
        }
    }

    #[inline]
    pub fn next(&self, rect: Rect) -> Self {
        Self {
            bounds: Rect {
                x: self.bounds.x + rect.x,
                y: self.bounds.y + rect.y,
                width: rect.width,
                height: rect.height
            }
        }
    }
}
