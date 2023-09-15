use std::{
    fmt,
    any::Any,
    future::Future,
    marker::PhantomData,
    collections::VecDeque,
    borrow::Borrow
};

use tiny_skia::PixmapMut;
use nohash::IntMap;
use tokio::{
    sync::mpsc::Sender,
    task::JoinHandle
};

use crate::{
    geometry::{Rect, Size, Point},
    widget::{
        Element, Widget, AnyWidget,
        size_constraints::SizeConstraints
    },
    theme::Theme,
    renderer::{Renderer, TextInfo},
    wayland::MouseEvent
};

type WidgetId = u64;

pub struct Ui {
    ctx: UiCtx,
    root: Id,
    size: Size,
    renderer: Renderer
}

pub struct TaskResult {
    id: WidgetId,
    data: Box<dyn Any + Send>
}

#[derive(Clone, Debug)]
pub struct ValueSender<T: Send> {
    id: WidgetId,
    sender: Sender<TaskResult>,
    phantom: PhantomData<T>
}

pub struct TypedId<E: Element> {
    id: Id,
    message: fn(
        &mut <E::Widget as Widget>::State,
        &mut UpdateCtx,
        E::Message
    ),
    data: std::marker::PhantomData<E>
}

#[derive(Eq, PartialEq, Hash, Debug)]
pub struct Id(WidgetId);

pub struct UiCtx {
    pub theme: Theme,
    mouse_pos: Point,
    widgets: IntMap<WidgetId, WidgetState>,
    id_counter: u64,
    needs_redraw: bool,
    needs_layout: bool,
    parent_to_children: IntMap<WidgetId, Vec<WidgetId>>,
    child_to_parent: IntMap<WidgetId, WidgetId>,
    task_sender: Sender<TaskResult>
}

pub struct LayoutCtx<'a> {
    renderer: &'a mut Renderer,
    ui: &'a mut UiCtx
}

pub struct DrawCtx<'a> {
    pub renderer: &'a mut Renderer,
    ui: &'a mut UiCtx,
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

#[derive(Debug)]
pub enum Event {
    Mouse(MouseEvent)
}

struct WidgetState {
    widget: Box<dyn AnyWidget>,
    state: Box<dyn Any>,
    layout: Rect
}

impl Ui {
    pub fn new<E: Element>(
        task_sender: Sender<TaskResult>,
        theme: Theme,
        build: impl FnOnce() -> E
    ) -> Self {
        let mut ctx = UiCtx {
            theme,
            mouse_pos: Point::ZERO,
            widgets: IntMap::default(),
            // We start from 1 because we use 0 just below as a fake ID
            // in order to begin building the widget tree. Thus, the root
            // widget will have an ID of 1 and in practice we never use 0
            // after that, when walking the tree.
            id_counter: 1,
            needs_redraw: false,
            needs_layout: false,
            parent_to_children: IntMap::default(),
            child_to_parent: IntMap::default(),
            task_sender
        };

        let mut init_ctx = InitCtx {
            current: 0,
            ui: &mut ctx
        };

        let root = init_ctx.new_child(build());

        Self {
            root: root.into(),
            ctx,
            size: Size::ZERO,
            renderer: Renderer::new()
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
        if let Event::Mouse(MouseEvent::MouseMove(pos)) = event {
            self.ctx.mouse_pos = pos;
        }

        let mut ctx = UpdateCtx {
            ui: &mut self.ctx,
            current: self.root.0
        };

        ctx.event(&self.root, &event);
    }

    pub fn task_result(&mut self, result: TaskResult) {
        // Widget might have been removed while the task was executing.
        let Some(state) = self.ctx.widgets.get_mut(&result.id) else {
            return;
        };

        let state = state as *mut WidgetState;

        let mut ctx = UpdateCtx {
            ui: &mut self.ctx,
            current: result.id
        };

        unsafe {
            let state = &mut (*state);
            state.widget.task_result(&mut state.state, &mut ctx, result.data);
        }
    }

    pub fn draw<'a: 'b, 'b>(&'a mut self, pixmap: &'b mut PixmapMut<'b>) {
        if self.ctx.needs_layout {
            self.layout_impl();
        }

        let mut ctx = DrawCtx {
            ui: &mut self.ctx,
            layout: Rect::default(),
            renderer: &mut self.renderer
        };

        ctx.draw(&self.root);

        self.ctx.needs_redraw = false;
        self.ctx.needs_layout = false;

        self.renderer.render(pixmap);
    }

    #[inline]
    pub fn needs_redraw(&self) -> bool {
        self.ctx.needs_redraw
    }

    #[inline]
    pub fn set_scale_factor(&mut self, scale_factor: f32) {
        if self.renderer.scale_factor() != scale_factor {
            self.renderer.set_scale_factor(scale_factor);
            self.ctx.needs_redraw = true;
        }
    }

    fn layout_impl(&mut self) {
        let mut ctx = LayoutCtx {
            renderer: &mut self.renderer,
            ui: &mut self.ctx
        };

        ctx.layout(&self.root, SizeConstraints::tight(self.size));

        // Translate all widget positions from parent local space
        // to window space. This feels like a giant hack but enables
        // granular updates as otherwise the only way to translate
        // to window space during draw would be to walk the widget
        // tree for EACH widget that is to be redrawn/updated. So we do this
        // only once here instead.
        //
        // Is there a better way to do this?
        let mut queue = VecDeque::with_capacity(
            self.ctx.parent_to_children.len()
        );
        queue.push_back(self.root.0);

        while let Some(current) = queue.pop_front() {
            // The entry will be None when we reach a leaf node.
            let children = self.ctx.parent_to_children.entry(current).or_default();
            queue.extend(children.iter());

            let offset = self.ctx.widgets[&current].layout.origin();
            
            for child in children {
                let state = self.ctx.widgets.get_mut(child).unwrap();
    
                state.layout.x += offset.x;
                state.layout.y += offset.y;
            }
        }
    }
}

impl UiCtx {
    fn dealloc(&mut self, id: Id) {
        let widget = self.widgets.remove(&id.0).unwrap();
        widget.widget.destroy(widget.state);

        let parent = self.child_to_parent.remove(&id.0).unwrap();
        let children = self.parent_to_children.get_mut(&parent).unwrap();
        let index = children.iter().position(|x| *x == id.0).unwrap();

        // Can't use swap_remove() because order is important
        children.remove(index);

        let children = self.parent_to_children
            .remove(&id.0)
            .unwrap_or_default();

        for child in children {
            self.dealloc_impl(child);
        }
    }

    fn dealloc_impl(&mut self, id: WidgetId) {
        let widget = self.widgets.remove(&id).unwrap();
        widget.widget.destroy(widget.state);

        self.child_to_parent.remove(&id).unwrap();

        let children = self.parent_to_children
            .remove(&id)
            .unwrap_or_default();

        for child in children {
            self.dealloc_impl(child);
        }
    }

    #[inline]
    fn next_id(&mut self) -> WidgetId {
        let id = self.id_counter;
        self.id_counter += 1;

        id
    }
}

impl<'a> LayoutCtx<'a> {
    #[inline]
    pub fn layout(&mut self, id: impl Borrow<Id>, bounds: SizeConstraints) -> Size {
        let state = self.ui.widgets.get_mut(&id.borrow().0)
            .unwrap() as *mut WidgetState;

        unsafe {
            let state = &mut (*state);
            state.layout = Rect::default();
            
            let size = state.widget.layout(&mut state.state, self, bounds);
            state.layout.set_size(size);

            size
        }
    }

    #[inline]
    pub fn measure_text(&mut self, info: &TextInfo, size: Size) -> Size {
        self.renderer.text_renderer.measure(info, size)
    }

    #[inline]
    pub fn position(
        &mut self,
        id: impl Borrow<Id>,
        func: impl FnOnce(&mut Rect)
    ) -> Rect {
        let state = self.ui.widgets.get_mut(&id.borrow().0).unwrap();
        func(&mut state.layout);

        state.layout
    }
}

impl<'a> UpdateCtx<'a> {
    #[inline]
    pub fn event(&mut self, id: impl Borrow<Id>, event: &Event) {
        let id = id.borrow().0;
        let state = self.ui.widgets.get_mut(&id)
            .unwrap() as *mut WidgetState;

        unsafe {
            let state = &mut (*state);

            let prev = self.current;
            self.current = id;

            // Can't use "state" after this as the map might have been resized.
            state.widget.event(&mut state.state, self, event);
            self.current = prev;
        }
    }

    #[inline]
    pub fn layout(&self) -> Rect {
        self.ui.widgets[&self.current].layout
    }

    #[inline]
    pub fn is_hovered(&self) -> bool {
        self.layout().contains(self.ui.mouse_pos)
    }

    #[inline]
    pub fn message<E: Element>(&mut self, id: &TypedId<E>, msg: E::Message) {
        let state = self.ui.widgets.get_mut(&id.inner())
            .unwrap() as *mut WidgetState;

        let prev = self.current;
        self.current = id.inner();

        unsafe {
            let state = &mut (*state);
            (id.message)(state.state.downcast_mut().unwrap(), self, msg);
        }

        self.current = prev;
    }

    #[inline]
    pub fn new_child<E: Element>(&mut self, el: E) -> TypedId<E>
        where E::Widget: AnyWidget
    {
        self.request_layout();
        
        let mut ctx = InitCtx {
            current: self.current,
            ui: self.ui
        };

        ctx.new_child(el)
    }

    /// Destroys the given widget and all its children **immediately**.
    #[inline]
    pub fn destroy_child(&mut self, id: impl Into<Id>) {
        self.request_layout();
        self.ui.dealloc(id.into());
    }

    #[inline]
    pub fn request_redraw(&mut self) {
        self.ui.needs_redraw = true;
    }

    #[inline]
    pub fn request_layout(&mut self) {
        self.ui.needs_layout = true;
        self.ui.needs_redraw = true;
    }
}

impl<'a> DrawCtx<'a> {
    #[inline]
    pub fn draw(&mut self, id: impl Borrow<Id>) {
        let state = self.ui.widgets.get_mut(&id.borrow().0)
            .unwrap() as *mut WidgetState;

        unsafe {
            let state = &mut (*state);

            let prev = self.layout;
            self.layout = state.layout;

            state.widget.draw(&mut state.state, self);

            self.layout = prev;
        }
    }

    #[inline]
    pub fn is_hovered(&self) -> bool {
        self.layout.contains(self.ui.mouse_pos)
    }

    #[inline]
    pub fn layout(&self) -> Rect {
        self.layout
    }
}

impl<'a> InitCtx<'a> {
    pub fn new_child<E: Element>(&mut self, el: E) -> TypedId<E>
        where E::Widget: AnyWidget
    {
        let child = self.ui.next_id();

        let parent = self.current;
        self.current = child;

        let (widget, state) = el.make_widget(self);

        self.current = parent;

        let state = WidgetState::new(Box::new(widget), Box::new(state));
        self.ui.widgets.insert(child, state);

        self.ui.child_to_parent.insert(child, parent);
        self.ui.parent_to_children.entry(parent)
            .or_default()
            .push(child);

        TypedId {
            id: Id(child),
            message: E::message,
            data: PhantomData
        }
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
        #[inline]
        #[doc = r"A fire and forget type task that does not produce any result.
This will NOT call [`Widget::task_result`] when complete."]
        #[must_use = r"It is your responsibility to abort long running or infinite loop
tasks if you don't need them anymore using the handle returned by this method.
You can ignore the return value otherwise."]
        pub fn task_void(
            &self,
            task: impl Future<Output = ()> + Send + 'static
        ) -> JoinHandle<()> {
            tokio::spawn(task)
        }

        #[doc = r"A task that produces a single value and when complete calls
[`Widget::task_result`] on the widget that initiated this method with the
value produced by the async computation. You MUST implement
[`Widget::task_result`] if you are using this method in your widget. If you
don't, the default implementation is a panic which will remind you of that."]
#[must_use = r"It is your responsibility to abort long running or infinite loop
tasks if you don't need them anymore using the handle returned by this method.
You can ignore the return value otherwise."]
        pub fn task<T: Send + 'static>(
            &self,
            task: impl Future<Output = T> + Send + 'static
        ) -> JoinHandle<()> {
            let tx = self.ui.task_sender.clone();
            let id = self.current;
    
            tokio::spawn(async move {
                let result = task.await;
                
                tx.send(TaskResult {
                    id,
                    data: Box::new(result)
                }).await.unwrap();
            })
        }
    
        #[doc = r"A task that can produce multiple values. For each value produced
[`Widget::task_result`] is called on the widget that initiated this method
with the value sent by the `ValueSender`. You MUST implement
[`Widget::task_result`] if you are using this method in your widget. If you
don't, the default implementation is a panic which will remind you of that."]
#[must_use = r"It is your responsibility to abort long running or infinite loop
tasks if you don't need them anymore using the handle returned by this method.
You can ignore the return value otherwise."]
        pub fn task_with_sender<T: Send + 'static, Fut>(
            &self,
            create_future: impl FnOnce(ValueSender<T>) -> Fut
        ) -> JoinHandle<()>
            where Fut: Future<Output = ()> + Send + 'static
        {
            let sender = ValueSender::new(
                self.current,
                self.ui.task_sender.clone()
            );
    
            tokio::spawn(create_future(sender))
        }
    }
}

impl_context_method! {
    InitCtx<'_>,
    LayoutCtx<'_>,
    UpdateCtx<'_>,
    DrawCtx<'_>,
    {
        #[inline]
        pub fn mouse_pos(&self) -> Point {
            self.ui.mouse_pos
        }

        #[inline]
        pub fn theme(&self) -> &Theme {
            &self.ui.theme
        }
    }
}

impl WidgetState {
    #[inline]
    fn new(widget: Box<dyn AnyWidget>, state: Box<dyn Any>) -> Self {
        Self {
            widget,
            state,
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
            data: Box::new(value)
        };

        self.sender.send(result).await.unwrap()
    }
}

impl<E: Element> TypedId<E> {
    #[inline]
    fn inner(&self) -> WidgetId {
        self.id.0
    }
}

impl<E: Element> Into<Id> for TypedId<E> {
    #[inline]
    fn into(self) -> Id {
        Id(self.inner())
    }
}

impl<E: Element> Borrow<Id> for TypedId<E> {
    #[inline]
    fn borrow(&self) -> &Id {
        &self.id
    }
}

impl<E: Element> Borrow<Id> for &TypedId<E> {
    #[inline]
    fn borrow(&self) -> &Id {
        &self.id
    }
}

impl<E: Element> Borrow<Id> for &mut TypedId<E> {
    #[inline]
    fn borrow(&self) -> &Id {
        &self.id
    }
}

impl fmt::Debug for TaskResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TaskResult")
            .field("id", &self.id)
            .field("data", &self.data)
            .finish()
    }
}
