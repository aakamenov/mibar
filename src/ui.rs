use std::mem;

use tiny_skia::{PixmapMut, PathBuilder};
use nohash::IntMap;

use crate::{
    geometry::{Rect, Circle, Size, Point},
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
    needs_layout: bool,
    widgets_to_redraw: Vec<WidgetId>,
    parent_to_children: IntMap<WidgetId, Vec<WidgetId>>,
    child_to_parent: IntMap<WidgetId, WidgetId>
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

pub struct InitCtx<'a> {
    ui: &'a mut UiCtx
}

#[derive(Debug)]
pub enum Event {
    Mouse(MouseEvent)
}

#[derive(Default, Debug)]
pub struct ChildWidgets(Vec<WidgetId>);

struct WidgetState {
    widget: Box<dyn Widget>,
    layout: Rect
}

impl Ui {
    pub fn new(
        builder: impl FnOnce(&mut InitCtx) -> Id
    ) -> Self {
        let mut ctx = UiCtx {
            theme: Theme::light(),
            widgets: IntMap::default(),
            id_counter: 0,
            needs_redraw: false,
            needs_layout: false,
            widgets_to_redraw: Vec::new(),
            parent_to_children: IntMap::default(),
            child_to_parent: IntMap::default()
        };

        let mut init_ctx = InitCtx { ui: &mut ctx };
        let root = builder(&mut init_ctx);

        // Build the widget tree by walking the children of each widget.
        let mut stack = vec![root.0];

        while let Some(current) = stack.pop() {
            let children = ctx.widgets[&current].widget.children().0;
            stack.extend_from_slice(&children);

            for child in &children {
                ctx.child_to_parent.insert(*child, current);
            }

            ctx.parent_to_children.insert(current, children);
        }

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

        self.layout_impl();
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

        // TODO: The background should be drawn by the root widget.
        pixmap.fill(self.ctx.theme.base);

        if self.ctx.needs_layout {
            self.layout_impl();
        }

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

        if ctx.ui.needs_layout || ctx.ui.widgets_to_redraw.is_empty() {
            // We do full layout and redraw at the moment if layout was requested.
            ctx.draw(&self.root);
        } else {
            let to_redraw = mem::take(&mut ctx.ui.widgets_to_redraw);

            for widget in to_redraw {
                if ctx.ui.widgets.contains_key(&widget) {
                    ctx.draw(&Id(widget));
                }
            }
        }

        self.ctx.needs_redraw = false;
        self.ctx.needs_layout = false;
    }

    #[inline]
    pub fn needs_redraw(&self) -> bool {
        self.ctx.needs_redraw
    }

    fn layout_impl(&mut self) {
        let mut ctx = LayoutCtx {
            ui: &mut self.ctx
        };

        ctx.layout(&self.root, SizeConstraints::tight(self.size));

        // Translate all widget positions from parent local space
        // to window space. This feels like a giant hack but enables
        // granular redrawing as otherwise the only way to translate
        // to window space during draw would be to walk the widget
        // tree for EACH widget that is to be redrawn. So we do this
        // only once here instead.
        //
        // Is there a better way to do this?
        let mut offset = Point::ZERO;

        let mut stack = Vec::with_capacity(
            self.ctx.parent_to_children.len()
        );
        stack.push(self.root.0);

        while let Some(current) = stack.pop() {
            let children = &self.ctx.parent_to_children[&current];
            stack.extend(children);

            offset = self.ctx.widgets[&current].layout.origin();
            
            for child in children {
                let state = self.ctx.widgets.get_mut(child).unwrap();
    
                state.layout.x += offset.x;
                state.layout.y += offset.y;
            }
        }
    }
}

impl UiCtx {
    fn alloc_with_parent(
        &mut self,
        widget: Box<dyn Widget>,
        parent: WidgetId
    ) -> Id {
        let id = self.alloc(widget);

        self.child_to_parent.insert(id.0, parent);
        self.parent_to_children
            .entry(parent)
            .or_insert_with(|| Vec::new())
            .push(id.0);

        id
    }

    fn alloc(
        &mut self,
        widget: Box<dyn Widget>
    ) -> Id {
        let state = WidgetState::new(widget);
        self.widgets.insert(self.id_counter, state);

        let id = Id(self.id_counter);
        self.id_counter += 1;

        id
    }

    fn dealloc(&mut self, id: Id) {
        self.widgets.remove(&id.0);
        self.child_to_parent.remove(&id.0);

        let children = self.parent_to_children
            .remove(&id.0)
            .unwrap_or(Vec::new());

        for child in children {
            self.dealloc(Id(child));
        }
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
            state.layout = Rect::default();

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

            let prev = self.current;
            self.current = id.0;

            // Can't use "state" after this as the map might have been resized.
            state.widget.event(self, event);
            self.current = prev;
        }
    }

    #[inline]
    pub fn layout(&self) -> Rect {
        self.ui.widgets.get(&self.current).unwrap().layout
    }

    #[inline]
    pub fn alloc(&mut self, widget: impl Widget + 'static) -> Id {
        self.request_layout();

        self.ui.alloc_with_parent(Box::new(widget), self.current)
    }

    #[inline]
    pub fn dealloc(&mut self, id: Id) {
        self.request_layout();
        self.ui.dealloc(id);
    }

    #[inline]
    pub fn request_redraw(&mut self) {
        // If layout was requested we layout and
        // then redraw all widgets currently.
        if !self.ui.needs_layout {
            self.ui.queue_draw(self.current);
        }
    }

    pub fn request_layout(&mut self) {
        self.ui.needs_layout = true;
        self.ui.needs_redraw = true;
        self.ui.widgets_to_redraw.clear();
    }
}

impl<'a, 'b> DrawCtx<'a, 'b> {
    #[inline]
    pub fn draw(&mut self, id: &Id) {
        let state = self.ui.widgets.get_mut(&id.0)
            .unwrap() as *mut WidgetState;

        unsafe {
            let state = &mut (*state);

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
        self.renderer.clip_stack.push(rect);
    }

    #[inline]
    pub fn pop_clip(&mut self) {
        self.renderer.clip_stack.pop();
    }
}

impl<'a> InitCtx<'a> {
    #[inline]
    pub fn alloc(&mut self, widget: impl Widget + 'static) -> Id {
        self.ui.alloc(Box::new(widget))
    }
}

impl WidgetState {
    #[inline]
    fn new(widget: Box<dyn Widget>) -> Self {
        Self {
            widget,
            layout: Rect::default()
        }
    }
}

impl ChildWidgets {
    #[inline]
    pub fn from_iter<'a>(
        children: impl Iterator<Item = &'a Id>
    ) -> Self {
        Self(Vec::from_iter(children.map(|x| x.0)))
    }
}

impl From<&Id> for ChildWidgets {
    #[inline]
    fn from(value: &Id) -> Self {
        Self(vec![value.0])
    }
}
