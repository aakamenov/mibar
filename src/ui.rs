use std::{
    mem,
    fmt,
    any::Any,
    future::Future,
    marker::PhantomData,
    collections::VecDeque
};

use tiny_skia::{PixmapMut, PathBuilder};
use nohash::IntMap;
use tokio::sync::mpsc::Sender;

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

pub struct TaskResult {
    id: WidgetId,
    result: Box<dyn Any + Send>
}

#[derive(Clone, Debug)]
pub struct ValueSender<T: Send> {
    id: WidgetId,
    sender: Sender<TaskResult>,
    phantom: PhantomData<T>
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
    child_to_parent: IntMap<WidgetId, WidgetId>,
    task_sender: Sender<TaskResult>
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
    ui: &'a mut UiCtx,
    current: WidgetId
}

pub struct CreateCtx<'a> {
    ui: &'a mut UiCtx
}

pub enum Event {
    Mouse(MouseEvent),
    TaskResult(Box<dyn Any>)
}

struct WidgetState {
    widget: Box<dyn Widget>,
    layout: Rect
}

impl Ui {
    pub fn new(
        task_sender: Sender<TaskResult>,
        builder: impl FnOnce(&mut CreateCtx) -> Id
    ) -> Self {
        let mut ctx = UiCtx {
            theme: Theme::light(),
            widgets: IntMap::default(),
            id_counter: 0,
            needs_redraw: false,
            needs_layout: false,
            widgets_to_redraw: Vec::new(),
            parent_to_children: IntMap::default(),
            child_to_parent: IntMap::default(),
            task_sender
        };

        let mut create_ctx = CreateCtx { ui: &mut ctx };
        let root = builder(&mut create_ctx);

        // Build the widget tree by walking the children of each widget.
        let state = ctx.widgets.get_mut(&root.0)
            .unwrap() as *mut WidgetState;

        let mut init_ctx = InitCtx {
            current: root.0,
            ui: &mut ctx
        };

        unsafe {
            let state = &mut (*state);
            state.widget.init(&mut init_ctx);
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

    pub fn task_result(&mut self, result: TaskResult) {
        // Widget might have been removed while the task was executing.
        if !self.ctx.widgets.contains_key(&result.id) {
            return;
        }

        let mut ctx = UpdateCtx {
            ui: &mut self.ctx,
            current: result.id
        };

        ctx.event(&Id(result.id), &Event::TaskResult(result.result));
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

        let mut queue = VecDeque::with_capacity(
            self.ctx.parent_to_children.len()
        );
        queue.push_back(self.root.0);

        while let Some(current) = queue.pop_front() {
            let children = &self.ctx.parent_to_children[&current];
            queue.extend(children);

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
    pub fn new_child(&mut self, widget: impl Widget + 'static) -> Id {
        self.request_layout();

        let id = self.ui.alloc(Box::new(widget));
        let mut ctx = InitCtx {
            current: self.current,
            ui: self.ui
        };

        ctx.init(&id);

        id
    }

    #[inline]
    pub fn dealloc_child(&mut self, id: Id) {
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
    pub fn init(&mut self, child: &Id) {
        let state = self.ui.widgets.get_mut(&child.0)
            .unwrap() as *mut WidgetState;

        self.ui.child_to_parent.insert(child.0, self.current);

        // Initialize an empty Vec because if the child is a leaf
        // node the entry in the map will never be initialized but
        // we want to have it so that we can conveniently walk the tree.
        self.ui.parent_to_children.insert(child.0, Vec::new());
        self.ui.parent_to_children.entry(self.current)
            .or_default()
            .push(child.0);

        unsafe {
            let state = &mut (*state);

            let prev = self.current;
            self.current = child.0;

            state.widget.init(self);
            self.current = prev;
        }
    }
}

impl<'a> CreateCtx<'a> {
    #[inline]
    pub fn alloc(&mut self, widget: impl Widget + 'static) -> Id {
        self.ui.alloc(Box::new(widget))
    }
}

// Copied from Xilem:
// https://github.com/linebender/xilem/blob/0759de95bd1f20bd28c84b517177c5b9a7aa4c61/src/widget/contexts.rs#L110
macro_rules! impl_context_method {
    ($ty:ty,  { $($method:item)+ } ) => {
        impl $ty { $($method)+ }
    };
    
    ( $ty:ty, $($more:ty),+, { $($method:item)+ } ) => {
        impl_context_method!($ty, { $($method)+ });
        impl_context_method!($($more),+, { $($method)+ });
    };
}

impl_context_method! {
    InitCtx<'_>,
    UpdateCtx<'_>,
    {
        pub fn task<T: Send + 'static>(
            &self,
            task: impl Future<Output = T> + Send + 'static
        ) {
            let tx = self.ui.task_sender.clone();
            let id = self.current;
    
            tokio::spawn(async move {
                let result = task.await;
                
                tx.send(TaskResult {
                    id,
                    result: Box::new(result)
                }).await.unwrap();
            });
        }
    
        pub fn task_with_sender<T: Send + 'static, Fut>(
            &self,
            create_future: impl FnOnce(ValueSender<T>) -> Fut
        ) where Fut: Future<Output = ()> + Send + 'static {
            let sender = ValueSender::new(
                self.current,
                self.ui.task_sender.clone()
            );
    
            tokio::spawn(create_future(sender));
        }
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

impl<T: Send + 'static> ValueSender<T> {
    #[inline]
    fn new(id: WidgetId, sender: Sender<TaskResult>) -> Self {
        Self {
            id,
            sender,
            phantom: PhantomData
        }
    }

    pub async fn send(&self, value: T) {
        let result = TaskResult {
            id: self.id,
            result: Box::new(value)
        };

        self.sender.send(result).await.unwrap()
    }
}

impl fmt::Debug for TaskResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TaskResult")
            .field("id", &self.id)
            .field("result", &self.result)
            .finish()
    }
}

impl fmt::Debug for Event {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Mouse(arg0) => f.debug_tuple("Mouse")
                .field(arg0)
                .finish(),
            Self::TaskResult(arg0) => f.write_fmt(
                format_args!("Task result: {:?}", arg0.type_id())
            )
        }
    }
}
