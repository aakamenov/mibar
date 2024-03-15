use std::{
    any::Any,
    marker::PhantomData,
    collections::VecDeque,
    hash::{Hash, Hasher},
    rc::Rc,
    cell::RefCell,
    fmt
};

use tiny_skia::PixmapMut;
use tokio::{runtime, task::JoinHandle, sync::mpsc::UnboundedSender};
use smithay_client_toolkit::reexports::calloop::channel::Sender;
use bumpalo::Bump;
use smallvec::SmallVec;

use crate::{
    geometry::{Rect, Size, Point},
    widget_tree::{WidgetTree, RawWidgetId, StateHandle, ContextHandle},
    event_queue::{EventQueue, EventType},
    widget::{Element, Widget, AnyWidget, SizeConstraints},
    theme::Theme,
    renderer::Renderer,
    wayland::{
        popup::{self, PopupWindowConfig},
        WindowEvent, MouseEvent, WindowConfig
    },
    client::{UiRequest, WindowId, WindowAction},
    window::Window,
    task::AsyncTask
};

pub struct TaskResult {
    id: RawWidgetId,
    data: Box<dyn Any + Send>
}

#[derive(Clone, Debug)]
pub struct ValueSender<T: Send> {
    id: RawWidgetId,
    sender: Sender<TaskResult>,
    phantom: PhantomData<T>
}

pub struct TypedId<T: Element> {
    id: RawWidgetId,
    data: PhantomData<T>
}

#[derive(Clone, Copy, Eq, PartialEq, Hash, Default, Debug)]
pub struct Id(pub(crate) RawWidgetId);

pub struct Context<'a> {
    pub ui: &'a mut UiCtx,
    pub tree: &'a mut WidgetTree,
    pub event_queue: &'a EventQueue<'a>
}

pub struct LayoutCtx<'a> {
    pub ui: &'a mut UiCtx,
    pub tree: &'a mut WidgetTree,
    pub renderer: &'a mut Renderer
}

pub struct DrawCtx<'a> {
    pub ui: &'a mut UiCtx,
    pub tree: &'a mut WidgetTree,
    pub renderer: &'a mut Renderer
}

#[derive(Debug)]
pub enum Event {
    Mouse(MouseEvent)
}

pub struct UiCtx {
    // Each Ui keeps a local copy of the current Theme. Whenever the theme
    // is mutated, the Ui sends a request to the client which then propagates
    // the changes to all the other windows. This may be more expensive than
    // using a mutex but changing the theme in practice happens rarely (if ever)
    // as opposed to synchronizing access every time we want to read it which
    // occurs multiple times per UI re-draw.
    theme: Theme,
    mouse_pos: Option<Point>,
    needs_redraw: bool,
    needs_layout: bool,
    rt_handle: runtime::Handle,
    task_send: Sender<TaskResult>,
    client_send: UnboundedSender<UiRequest>,
    window_id: WindowId
}

#[derive(Debug)]
pub(crate) enum UiEvent {
    Window(WindowEvent),
    Task(TaskResult)
}

pub(crate) struct Ui {
    pub(crate) renderer: Renderer,
    pub(crate) ctx: UiCtx,
    alloc: Bump,
    tree: WidgetTree,
    root: Id,
    size: Size
}

pub(crate) struct UiConfig {
    pub window_id: WindowId,
    pub theme: Theme,
    pub build_ui: fn(&mut Context) -> Id,
    pub rt_handle: runtime::Handle,
    pub client_send: UnboundedSender<UiRequest>
}

impl Ui {
    pub fn new(
        UiConfig {
            window_id,
            theme,
            build_ui,
            rt_handle,
            client_send
        }: UiConfig,
        task_send: Sender<TaskResult>
    ) -> Self {
        let mut tree = WidgetTree::new();
        let mut alloc = Bump::new();

        let mut ctx = UiCtx {
            theme,
            mouse_pos: None,
            needs_redraw: false,
            needs_layout: false,
            rt_handle,
            task_send,
            client_send,
            window_id
        };

        let buffer = RefCell::new(Vec::with_capacity(8));

        let mut init_ctx = Context {
            tree: &mut tree,
            ui: &mut ctx,
            event_queue: &EventQueue {
                alloc: &alloc,
                buffer: &buffer
            }
        };

        let root = build_ui(&mut init_ctx);
        init_ctx.execute_events();
        alloc.reset();

        Self {
            ctx,
            renderer: Renderer::new(),
            alloc,
            tree,
            root: root.into(),
            size: Size::ZERO
        }
    }

    pub fn set_theme(&mut self, theme: Theme) {
        self.ctx.theme = theme;
        self.ctx.needs_redraw = true;
    }

    /// Set a new `size` that the ui **has** to accommodate.
    pub fn set_size(&mut self, size: Size) {
        if size == self.size {
            return;
        }

        self.size = size;
        self.ctx.needs_redraw = true;

        self.layout_impl(SizeConstraints::tight(size));
    }

    /// Perform a layout using the given `size` which will serve as the maximum allowed space that the ui can use.
    /// Returns the actual size that the ui content will fit in.
    pub fn layout(&mut self, size: Size) -> Size {
        let new_size = self.layout_impl(SizeConstraints::tight(size).loosen());

        if new_size != self.size {
            self.ctx.needs_redraw = true;
            self.size = new_size;
        }

        self.size
    }

    pub fn event(&mut self, event: Event) {
        match event {
            Event::Mouse(MouseEvent::MouseMove(pos)) =>
                self.ctx.mouse_pos = Some(pos),
            Event::Mouse(MouseEvent::LeaveWindow) =>
                self.ctx.mouse_pos = None,
            _ => { }
        }

        let buffer = RefCell::new(Vec::with_capacity(8));

        let mut ctx = Context {
            tree: &mut self.tree,
            ui: &mut self.ctx,
            event_queue: &EventQueue {
                alloc: &self.alloc,
                buffer: &buffer
            }
        };

        ctx.event(self.root, &event);
        ctx.execute_events();
        self.alloc.reset();
    }

    pub fn task_result(&mut self, result: TaskResult) {
        // Widget might have been removed while the task was executing.
        let Some(state) = self.tree.widgets.get(result.id) else {
            return;
        };

        let widget = Rc::clone(&state.widget);
        let buffer = RefCell::new(Vec::with_capacity(8));

        let mut ctx = Context {
            tree: &mut self.tree,
            ui: &mut self.ctx,
            event_queue: &EventQueue {
                alloc: &self.alloc,
                buffer: &buffer
            }
        };

        widget.task_result(Id(result.id), &mut ctx, result.data);
        ctx.execute_events();
        self.alloc.reset();
    }

    pub fn draw<'a: 'b, 'b>(&'a mut self, pixmap: &'b mut PixmapMut<'b>) {
        if self.ctx.needs_layout {
            self.layout_impl(SizeConstraints::tight(self.size));
        }

        let mut ctx = DrawCtx {
            renderer: &mut self.renderer,
            tree: &mut self.tree,
            ui: &mut self.ctx
        };

        ctx.draw(self.root);

        self.ctx.needs_redraw = false;
        self.ctx.needs_layout = false;

        self.renderer.render(pixmap);
    }

    pub fn destroy(mut self) {
        self.tree.dealloc(self.root.0);
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

    fn layout_impl(&mut self, constraints: SizeConstraints) -> Size {
        let mut ctx = LayoutCtx {
            renderer: &mut self.renderer,
            tree: &mut self.tree,
            ui: &mut self.ctx
        };

        let size = ctx.layout(self.root, constraints);
        self.tree.widgets[self.root.0].layout.set_size(size);

        // Translate all widget positions from parent local space
        // to window space. This feels like a giant hack but enables us to have
        // a separate draw step as otherwise the only way to translate
        // to window space during draw would be to walk the widget
        // tree for EACH widget that is to be redrawn/updated. So we do this
        // only once here instead.
        //
        // Is there a better way to do this?
        let mut queue = VecDeque::with_capacity(
            self.tree.parent_to_children.len()
        );
        queue.push_back(self.root.0);

        while let Some(current) = queue.pop_front() {
            // The entry will be None when we reach a leaf node.
            let children = self.tree.parent_to_children.entry(current).unwrap().or_default();
            queue.extend(children.iter());

            let offset = self.tree.widgets[current].layout.origin();
            
            for child in children {
                let state = self.tree.widgets.get_mut(*child).unwrap();
    
                state.layout.x += offset.x;
                state.layout.y += offset.y;
            }
        }

        size
    }
}

impl UiCtx {
    #[inline]
    pub fn request_redraw(&mut self) {
        self.needs_redraw = true;
    }

    #[inline]
    pub fn request_layout(&mut self) {
        self.needs_layout = true;
        self.needs_redraw = true;
    }

    /// `None` means the mouse is currently outside the window.
    #[inline]
    pub fn mouse_pos(&self) -> Option<Point> {
        self.mouse_pos
    }

    #[inline]
    pub fn is_hovered(&self, layout: Rect) -> bool {
        if let Some(pos) = self.mouse_pos {
            layout.contains(pos)
        } else {
            false
        }
    }

    #[inline]
    pub fn theme(&self) -> &Theme {
        &self.theme
    }

    pub fn theme_mut(&mut self, change: impl FnOnce(&mut Theme)) {
        change(&mut self.theme);
        self.needs_redraw = true;

        self.client_send.send(
            UiRequest {
                id: self.window_id,
                action: WindowAction::ThemeChanged(self.theme.clone())
            }
        ).unwrap();
    }

    #[inline]
    pub fn window_id(&self) -> WindowId {
        self.window_id
    }

    #[inline]
    pub fn runtime_handle(&self) -> &runtime::Handle {
        &self.rt_handle
    }

    #[inline]
    pub fn value_sender<T: Send + 'static>(&self, id: impl Into<Id>) -> ValueSender<T> {
        ValueSender::new(
            id.into().0,
            self.task_send.clone()
        )
    }

    #[inline]
    #[must_use = r"It is your responsibility to abort long running or infinite loop
tasks if you don't need them anymore using the handle returned by this method.
You can ignore the return value otherwise."]
    pub fn spawn<T: AsyncTask>(&self, task: T) -> JoinHandle<()> {
        task.spawn(self)
    }
}

impl<'a> LayoutCtx<'a> {
    #[inline]
    pub fn layout(&mut self, id: impl Into<Id>, bounds: SizeConstraints) -> Size {
        let id = id.into().0;

        let state = &mut self.tree.widgets[id];
        state.layout = Rect::default();

        let widget = Rc::clone(&state.widget);

        let size = widget.layout(Id(id), self, bounds);
        self.tree.widgets[id].layout.set_size(size);

        size
    }

    #[inline]
    pub fn position(
        &mut self,
        id: impl Into<Id>,
        func: impl FnOnce(&mut Rect)
    ) -> Rect {
        let state = &mut self.tree.widgets[id.into().0];
        func(&mut state.layout);

        state.layout
    }
}

impl<'a> Context<'a> {
    #[inline]
    pub fn event(&mut self, id: impl Into<Id>, event: &Event) {
        let id = id.into();
        let state = &mut self.tree.widgets[id.0];

        let widget = Rc::clone(&state.widget);

        widget.event(id, self, event);
    }

    pub fn new_child<E: Element>(&mut self, parent: impl Into<Id>, el: E) -> TypedId<E>
        where E::Widget: AnyWidget
    {
        let parent = parent.into().0;
        self.ui.request_layout();

        let child = el.make(self);
        self.tree.child_to_parent.insert(child.raw(), parent);

        let entry = self.tree.parent_to_children.entry(parent).unwrap();
        entry.or_default().push(child.raw());

        child
    }

    /// Destroys the given widget and all its children **immediately**.
    #[inline]
    pub fn destroy_child(&mut self, id: impl Into<Id>) {
        self.ui.request_layout();
        self.tree.dealloc(id.into().0);
    }

    pub fn with_context<T: 'static>(
        &mut self,
        context: T,
        f: impl FnOnce(&mut Self, ContextHandle<T>)
    ) -> T {
        let id = self.tree.contexts.insert(Box::new(context));
        f(self, ContextHandle::new(id));

        let context = self.tree.contexts.remove(id).unwrap();
        let boxed = context.downcast().unwrap();

        *boxed
    }

    pub fn open_window(
        &self,
        window: impl Into<Window>,
        build_ui: fn(&mut Context) -> Id
    ) -> WindowId {
        let id = WindowId::new();
        let config = match window.into() {
            Window::Bar(bar) => WindowConfig::LayerShell(bar.into()),
            Window::SidePanel(panel) => WindowConfig::LayerShell(panel.into()),
            Window::Popup(popup) => {
                let parent = self.ui.window_id()
                    .surface()
                    .expect("attempting to open a popup during Ui init");

                let anchor_rect = match popup.location {
                    popup::Location::Cursor => {
                        let pos = self.ui.mouse_pos().unwrap_or_default();
                        Rect::new(pos.x, pos.y, 1f32, 1f32)
                    }
                    popup::Location::WidgetBounds(id) =>
                        self.tree.widgets[id.0].layout,
                    popup::Location::Bounds(rect) => rect
                };

                WindowConfig::Popup(
                    PopupWindowConfig {
                        parent,
                        size: popup.size,
                        anchor: popup.anchor,
                        anchor_rect
                    }
                )
            }
        };

        self.ui.client_send.send(UiRequest {
            id,
            action: WindowAction::Open {
                config,
                build_ui
            }
        }).unwrap();

        id
    }

    #[inline]
    pub fn close_window(&self, id: WindowId) {
        self.ui.client_send.send(
            UiRequest { id, action: WindowAction::Close }
        ).unwrap();
    }

    fn execute_events(&mut self) {
        let mut events: SmallVec<[EventType<'a>; 16]> = SmallVec::new();
        events.extend(self.event_queue.buffer.borrow_mut().drain(..).rev());

        while let Some(event) = events.pop() {
            match event {
                EventType::Action(action) => {
                    // Safety: we only call this function once, this will also invoke the drop code
                    // and the memory will be released by the allocator at the end of the interaction.
                    unsafe { action.invoke(self); }
                    events.extend(self.event_queue.buffer.borrow_mut().drain(..).rev());
                }
                EventType::Destroy(ids) => for id in ids {
                    self.destroy_child(id)
                }
            }
        }
    }
}

impl<'a> DrawCtx<'a> {
    #[inline]
    pub fn draw(&mut self, id: impl Into<Id>) {
        let id = id.into().0;
        let state = &mut self.tree.widgets[id];

        let widget = Rc::clone(&state.widget);
        let layout = state.layout;

        widget.draw(Id(id), self, layout);
    }
}

impl<T: Send + 'static> ValueSender<T> {
    #[inline]
    fn new(id: RawWidgetId, sender: Sender<TaskResult>) -> Self {
        Self {
            id,
            sender,
            phantom: PhantomData
        }
    }

    #[inline]
    pub fn send(&self, value: T) -> bool {
        let result = TaskResult {
            id: self.id,
            data: Box::new(value)
        };

        self.sender.send(result).is_ok()
    }
}

impl<T: Send> Hash for ValueSender<T> {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl<T: Element> TypedId<T> {
    pub(crate) fn new(id: RawWidgetId) -> Self {
        Self { id, data: PhantomData }
    }

    #[inline]
    pub(crate) fn raw(&self) -> RawWidgetId {
        self.id
    }
}

impl<T: Element> Into<Id> for TypedId<T> {
    #[inline]
    fn into(self) -> Id {
        Id(self.raw())
    }
}

impl<T: Element> Into<StateHandle<<T::Widget as Widget>::State>> for TypedId<T> {
    #[inline]
    fn into(self) -> StateHandle<<T::Widget as Widget>::State> {
        StateHandle::new(self.id)
    }
}

impl<T: Element> Clone for TypedId<T> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            data: PhantomData,
        }
    }
}

impl<T: Element> Copy for TypedId<T> { }

impl<T: Element> fmt::Debug for TypedId<T> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.id, f)
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
