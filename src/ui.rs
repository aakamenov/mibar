use tiny_skia::{PixmapMut, PathBuilder};
use nohash::IntMap;

use crate::{
    geometry::{Rect, Circle, Size},
    positioner::Positioner,
    widget::{
        Widget,
        size_constraints::SizeConstraints
    },
    theme::Theme,
    renderer::{Renderer, Background}
};

type WidgetId = u64;

pub struct Ui {
    ctx: UiCtx,
    root: Id,
    size: Size
}

#[derive(Eq, PartialEq, Hash, Debug)]
pub struct Id(WidgetId);

pub struct UiCtx {
    pub theme: Theme,
    widgets: IntMap<WidgetId, Box<dyn Widget>>,
    id_counter: u64
}

pub struct LayoutCtx<'a> {
    ui: &'a mut UiCtx
}

pub struct DrawCtx<'a> {
    pub ui: &'a mut UiCtx,
    renderer: Renderer<'a>
}

impl Ui {
    pub fn new(
        builder: impl FnOnce(&mut UiCtx) -> Id
    ) -> Self {
        let mut ctx = UiCtx {
            theme: Theme::light(),
            widgets: IntMap::default(),
            id_counter: 0
        };

        let root = builder(&mut ctx);

        Self {
            root,
            ctx,
            size: Size::ZERO
        }
    }

    pub fn layout(&mut self, size: Size) {
        self.size = size;

        let mut ctx = LayoutCtx {
            ui: &mut self.ctx  
        };


        ctx.layout(&self.root, SizeConstraints::tight(size));
    }

    pub fn draw<'a: 'b, 'b>(&'a mut self, pixmap: &'b mut PixmapMut<'b>) {
        assert_eq!(pixmap.width() , self.size.width as u32);
        assert_eq!(pixmap.height() , self.size.height as u32);

        pixmap.fill(self.ctx.theme.base);

        let mut ctx = DrawCtx {
            ui: &mut self.ctx,
            renderer: Renderer {
                pixmap,
                builder: PathBuilder::new(),
                clip_stack: Vec::new()
            }
        };

        ctx.draw(&self.root, Positioner::new(self.size));
    }
}

impl UiCtx {
    #[inline]
    pub fn alloc(&mut self, widget: impl Widget + 'static) -> Id {
        self.widgets.insert(self.id_counter, Box::new(widget));

        let id = Id(self.id_counter);
        self.id_counter += 1;

        id
    }

    #[inline]
    pub fn dealloc(&mut self, id: Id) {
        self.widgets.remove(&id.0);
    }
}

impl<'a> LayoutCtx<'a> {
    #[inline]
    pub fn layout(&mut self, id: &Id, bounds: SizeConstraints) -> Size {
        let widget = self.ui.widgets.get_mut(&id.0)
            .unwrap()
            .as_mut() as *mut dyn Widget;

        unsafe {
            (*widget).layout(self, bounds)
        }
    }
}

impl<'a> DrawCtx<'a> {
    #[inline]
    pub fn draw(&mut self, id: &Id, positioner: Positioner) {
        let widget = self.ui.widgets.get_mut(&id.0)
            .unwrap()
            .as_mut() as *mut dyn Widget;

        unsafe {
            (*widget).draw(self, positioner);
        }
    }

    #[inline]
    pub fn fill_circle(&mut self, circle: Circle, bg: impl Into<Background>) {
        self.renderer.builder.push_circle(circle.x, circle.y, circle.radius);
        self.renderer.draw_path(bg);

    }

    #[inline]
    pub fn fill_rect(&mut self, rect: Rect, bg: impl Into<Background>) {
        self.renderer.builder.push_rect(rect.x, rect.y, rect.width, rect.height);
        self.renderer.draw_path(bg);
    }

    #[inline]
    pub fn push_clip(&mut self, rect: Rect) {
        self.renderer.clip_stack.push(rect)
    }

    #[inline]
    pub fn pop_clip(&mut self) {
        self.renderer.clip_stack.pop();
    }
}
