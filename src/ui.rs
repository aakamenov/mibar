use tiny_skia::{PixmapMut, PathBuilder};
use nohash::IntMap;

use crate::{
    geometry::{Rect, Point, Circle, Size},
    widget::{
        Widget,
        size_constraints::SizeConstraints
    },
    theme::Theme,
    renderer::{Renderer, Background}
};

type WidgetId = u64;

pub struct Ui {
    pub(crate) needs_redraw: bool,
    ctx: UiCtx,
    root: Id,
    size: Size
}

#[derive(Eq, PartialEq, Hash, Debug)]
pub struct Id(WidgetId);

pub struct UiCtx {
    pub theme: Theme,
    widgets: IntMap<WidgetId, WidgetState>,
    id_counter: u64
}

pub struct LayoutCtx<'a> {
    ui: &'a mut UiCtx
}

pub struct DrawCtx<'a, 'b> {
    pub ui: &'a mut UiCtx,
    renderer: &'a mut Renderer<'b>,
    layout: Rect
}

struct WidgetState {
    widget: Box<dyn Widget>,
    layout: Rect
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
            needs_redraw: false,
            root,
            ctx,
            size: Size::ZERO
        }
    }

    pub fn layout(&mut self, size: Size) {
        if size == self.size {
            return;
        }

        self.size = size;

        let mut ctx = LayoutCtx {
            ui: &mut self.ctx  
        };

        ctx.layout(&self.root, SizeConstraints::tight(size));
        self.needs_redraw = true;
    }

    pub fn draw<'a: 'b, 'b>(&'a mut self, pixmap: &'b mut PixmapMut<'b>) {
        assert_eq!(pixmap.width() , self.size.width as u32);
        assert_eq!(pixmap.height() , self.size.height as u32);

        pixmap.fill(self.ctx.theme.base);

        let mut renderer = Renderer {
            pixmap,
            builder: PathBuilder::new(),
            clip_stack: Vec::new()
        };

        let mut ctx = DrawCtx {
            ui: &mut self.ctx,
            layout: Rect::default(),
            renderer: &mut renderer
        };

        ctx.draw(&self.root);
        self.needs_redraw = false;
    }
}

impl UiCtx {
    #[inline]
    pub fn alloc(&mut self, widget: impl Widget + 'static) -> Id {
        let state = WidgetState::new(Box::new(widget));
        self.widgets.insert(self.id_counter, state);

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
        let state = self.ui.widgets.get_mut(&id.0)
            .unwrap() as *mut WidgetState;

        unsafe {
            let state = &mut (*state);

            let size = state.widget.layout(self, bounds);
            state.layout.set_size(size);

            size
        }
    }

    #[inline]
    pub fn set_origin(&mut self, id: &Id, point: impl Into<Point>) {
        let state = self.ui.widgets.get_mut(&id.0).unwrap();
        state.layout.set_origin(point);
    }

    #[inline]
    pub fn layout_of(&mut self, id: &Id) -> &mut Rect {
        &mut self.ui.widgets.get_mut(&id.0).unwrap().layout
    }
}

impl<'a, 'b> DrawCtx<'a, 'b> {
    #[inline]
    pub fn draw(&mut self, id: &Id) {
        let state = self.ui.widgets.get_mut(&id.0)
            .unwrap() as *mut WidgetState;

        unsafe {
            let state = &mut (*state);
            state.layout.x += self.layout.x;
            state.layout.y += self.layout.y;

            let prev = self.layout;
            self.layout = state.layout;

            state.widget.draw(self);

            self.layout = prev;
        }
    }

    #[inline]
    pub fn layout(&self) -> Rect {
        self.layout
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

impl WidgetState {
    fn new(widget: Box<dyn Widget>) -> Self {
        Self {
            widget,
            layout: Rect::default()
        }
    }
}
