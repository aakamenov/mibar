use std::mem;

use tiny_skia::{PixmapMut, PathBuilder};
use nohash::IntMap;

use crate::{
    geometry::{Rect, Circle, Size},
    widget::{
        Widget,
        size_constraints::SizeConstraints
    },
    theme::Theme,
    renderer::{Renderer, Background},
    wayland::MouseEvent
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
    widgets: IntMap<WidgetId, WidgetState>,
    id_counter: u64,
    needs_redraw: bool,
    widgets_to_redraw: Vec<WidgetId>
}

pub struct LayoutCtx<'a> {
    ui: &'a mut UiCtx
}

pub struct DrawCtx<'a, 'b> {
    pub ui: &'a mut UiCtx,
    renderer: &'a mut Renderer<'b>,
    layout: Rect
}

pub struct UpdateCtx<'a> {
    ui: &'a mut UiCtx,
    current: WidgetId
}

#[derive(Debug)]
pub enum Event {
    Mouse(MouseEvent)
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
            id_counter: 0,
            needs_redraw: false,
            widgets_to_redraw: Vec::new()
        };

        let root = builder(&mut ctx);

        Self {
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
        self.ctx.needs_redraw = true;

        let mut ctx = LayoutCtx {
            ui: &mut self.ctx
        };

        ctx.layout(&self.root, SizeConstraints::tight(size));
    }

    pub fn event(&mut self, event: Event) {
        let mut ctx = UpdateCtx {
            ui: &mut self.ctx,
            current: self.root.0
        };

        ctx.event(&self.root, &event);
    }

    pub fn draw<'a: 'b, 'b>(&'a mut self, pixmap: &'b mut PixmapMut<'b>) {
        assert_eq!(pixmap.width() , self.size.width as u32);
        assert_eq!(pixmap.height() , self.size.height as u32);

        pixmap.fill(self.ctx.theme.base);
        self.ctx.needs_redraw = false;

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

        if ctx.ui.widgets_to_redraw.len() > 0 {
            let to_redraw = mem::take(&mut ctx.ui.widgets_to_redraw);

            for widget in to_redraw {
                if ctx.ui.widgets.contains_key(&widget) {
                    ctx.draw(&Id(widget));
                }
            }
        } else {
            // Full re-draw after layout. Since widgets_to_redraw 
            // cannot be changed during layout we know that it occurred.
            ctx.draw(&self.root);
        }
    }

    #[inline]
    pub fn needs_redraw(&self) -> bool {
        self.ctx.needs_redraw
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

    #[inline]
    fn queue_draw(&mut self, id: WidgetId) {
        self.widgets_to_redraw.push(id);
        self.needs_redraw = true;
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
    pub fn position(
        &mut self,
        id: &Id,
        func: impl FnOnce(&mut Rect)
    ) -> Rect {
        let state = self.ui.widgets.get_mut(&id.0).unwrap();
        func(&mut state.layout);

        state.layout
    }
}

impl<'a> UpdateCtx<'a> {
    #[inline]
    pub fn event(&mut self, id: &Id, event: &Event) {
        let state = self.ui.widgets.get_mut(&id.0)
            .unwrap() as *mut WidgetState;

        unsafe {
            let state = &mut (*state);

            let modified_event = match event {
                Event::Mouse(mut event) => {
                    match &mut event {
                        MouseEvent::MousePress { pos, .. } |
                        MouseEvent::MouseRelease { pos, .. } |
                        MouseEvent::MouseMove(pos) => {
                            let layout = self.layout();
                            pos.x -= layout.x;
                            pos.y -= layout.y;

                            Some(Event::Mouse(event))
                        }
                    }
                }
            };

            let prev = self.current;
            self.current = id.0;

            state.widget.event(
                self,
                modified_event.as_ref().unwrap_or(event)
            );
            self.current = prev;
        }
    }

    #[inline]
    pub fn layout(&self) -> Rect {
        self.ui.widgets.get(&self.current).unwrap().layout
    }
}

impl<'a, 'b> DrawCtx<'a, 'b> {
    #[inline]
    pub fn draw(&mut self, id: &Id) {
        let state = self.ui.widgets.get_mut(&id.0)
            .unwrap() as *mut WidgetState;

        unsafe {
            let state = &mut (*state);

            let mut layout = state.layout;
            layout.x += self.layout.x;
            layout.y += self.layout.y;

            let prev = self.layout;
            self.layout = layout;

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

impl<'a> UpdateCtx<'a> {
    #[inline]
    pub fn request_redraw(&mut self) {
        self.ui.queue_draw(self.current);
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
