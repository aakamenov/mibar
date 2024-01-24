use std::{
    fmt,
    any::Any,
    future::Future,
    marker::PhantomData,
    collections::VecDeque,
    hash::{Hash, Hasher},
    rc::Rc
};

use tiny_skia::PixmapMut;
use tokio::{runtime, task::JoinHandle, sync::mpsc::UnboundedSender};
use smithay_client_toolkit::reexports::calloop::channel::Sender;
use slotmap::Key;

use crate::{
    geometry::{Rect, Size, Point},
    widget_tree::{
        WidgetTree, WidgetState, RawWidgetId,
        RawActionId, StateHandle, Action
    },
    widget::{Element, Widget, AnyWidget, SizeConstraints},
    theme::Theme,
    renderer::{Renderer, ImageCacheHandle},
    wayland::{
        popup::{self, PopupWindowConfig},
        WindowEvent, MouseEvent, WindowConfig
    },
    client::{UiRequest, WindowId, WindowAction},
    window::Window,
    asset_loader::{self, AssetSource}
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

#[derive(Debug)]
pub struct TypedId<E: Element> {
    id: Id,
    message: fn(
        StateHandle<<E::Widget as Widget>::State>,
        &mut UpdateCtx,
        E::Message
    ),
    data: PhantomData<E>
}

#[derive(Clone, Copy, Eq, PartialEq, Hash, Debug)]
pub struct Id(RawWidgetId);

#[derive(Clone, Copy, Eq, PartialEq, Hash, Debug)]
pub struct ActionId {
    widget: RawWidgetId,
    action: RawActionId
}

pub struct LayoutCtx<'a> {
    pub ui: &'a mut UiCtx,
    pub tree: &'a mut WidgetTree,
    pub renderer: &'a mut Renderer,
    pub(crate) active: RawWidgetId
}

pub struct DrawCtx<'a> {
    pub ui: &'a mut UiCtx,
    pub tree: &'a mut WidgetTree,
    pub renderer: &'a mut Renderer,
    pub(crate) active: RawWidgetId
}

pub struct UpdateCtx<'a> {
    pub ui: &'a mut UiCtx,
    pub tree: &'a mut WidgetTree,
    pub(crate) active: RawWidgetId
}

pub struct InitCtx<'a> {
    pub ui: &'a mut UiCtx,
    pub(crate) tree: &'a mut WidgetTree,
    pub(crate) active: RawWidgetId
}

#[derive(Debug)]
pub enum Event {
    Mouse(MouseEvent)
}

pub struct UiCtx {
    pub(crate) image_cache_handle: ImageCacheHandle,
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
    tree: WidgetTree,
    root: Id,
    size: Size
}

impl Ui {
    pub fn new<E: Element>(
        window_id: WindowId,
        rt_handle: runtime::Handle,
        task_send: Sender<TaskResult>,
        client_send: UnboundedSender<UiRequest>,
        theme: Theme,
        root: E
    ) -> Self {
        let mut renderer = Renderer::new();
        let mut tree = WidgetTree::new();
        let mut ctx = UiCtx {
            theme,
            mouse_pos: None,
            image_cache_handle: renderer.image_cache_handle(),
            needs_redraw: false,
            needs_layout: false,
            rt_handle,
            task_send,
            client_send,
            window_id
        };

        let mut init_ctx = InitCtx {
            tree: &mut tree,
            ui: &mut ctx,
            active: RawWidgetId::null()
        };

        let root = init_ctx.new_child(root);

        Self {
            ctx,
            renderer,
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

        let mut ctx = UpdateCtx {
            tree: &mut self.tree,
            ui: &mut self.ctx,
            active: RawWidgetId::null()
        };

        ctx.event(self.root, &event);
    }

    pub fn task_result(&mut self, result: TaskResult) {
        // Widget might have been removed while the task was executing.
        let Some(state) = self.tree.widgets.get(result.id) else {
            return;
        };

        let widget = Rc::clone(&state.widget);
        let mut ctx = UpdateCtx {
            tree: &mut self.tree,
            ui: &mut self.ctx,
            active: result.id
        };

        widget.task_result(&mut ctx, result.data);
    }

    pub fn draw<'a: 'b, 'b>(&'a mut self, pixmap: &'b mut PixmapMut<'b>) {
        if self.ctx.needs_layout {
            self.layout_impl(SizeConstraints::tight(self.size));
        }

        let mut ctx = DrawCtx {
            renderer: &mut self.renderer,
            tree: &mut self.tree,
            ui: &mut self.ctx,
            active: RawWidgetId::null()
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
            ui: &mut self.ctx,
            active: RawWidgetId::null()
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
}

impl<'a> LayoutCtx<'a> {
    #[inline]
    pub fn layout(&mut self, id: impl Into<Id>, bounds: SizeConstraints) -> Size {
        let next = id.into().0;

        let state = &mut self.tree.widgets[next];
        state.layout = Rect::default();

        let active = self.active;
        let widget = Rc::clone(&state.widget);

        self.active = next;

        let size = widget.layout(self, bounds);
        self.tree.widgets[next].layout.set_size(size);

        self.active = active;

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

impl<'a> UpdateCtx<'a> {
    #[inline]
    pub fn event(&mut self, id: impl Into<Id>, event: &Event) {
        let next = id.into().0;
        let state = &mut self.tree.widgets[next];

        let active = self.active;
        let widget = Rc::clone(&state.widget);

        self.active = next;
        widget.event(self, event);
        self.active = active;
    }

    #[inline]
    pub fn layout(&self) -> Rect {
        self.tree.widgets[self.active].layout
    }

    #[inline]
    pub fn is_hovered(&self) -> bool {
        let layout = self.layout();

        self.ui.is_hovered(layout)
    }

    #[inline]
    pub fn message<E: Element>(&mut self, id: TypedId<E>, msg: E::Message) {
        let next = id.raw();
        let active = self.active;

        self.active = next;
        (id.message)(StateHandle::new(next), self, msg);
        self.active = active;
    }

    #[inline]
    pub fn execute(&mut self, id: ActionId) -> bool {
        let Some(actions) = self.tree.actions.get(id.widget) else {
            return false;
        };

        if let Some(action) = actions.get(id.action).map(|x| Rc::clone(x)) {
            action(self);

            true
        } else {
            false
        }
    }

    #[inline]
    pub fn new_child<E: Element>(&mut self, el: E) -> TypedId<E>
        where E::Widget: AnyWidget
    {
        self.ui.request_layout();
        
        let mut ctx = InitCtx {
            active: self.active,
            tree: self.tree,
            ui: self.ui
        };

        ctx.new_child(el)
    }

    /// Destroys the given widget and all its children **immediately**.
    #[inline]
    pub fn destroy_child(&mut self, id: impl Into<Id>) {
        self.ui.request_layout();
        self.tree.dealloc(id.into().0);
    }

    pub fn open_window<E: Element>(
        &self,
        window: impl Into<Window>,
        root: impl FnOnce() -> E + Send + 'static
    ) -> WindowId {
        let id = WindowId::new();
        let make_ui = Box::new(move |theme, rt_handle, task_send, client_send| {
            Ui::new(id, rt_handle, task_send, client_send, theme, root())
        });

        let config = match window.into() {
            Window::Bar(bar) => WindowConfig::LayerShell(bar.into()),
            Window::SidePanel(panel) => WindowConfig::LayerShell(panel.into()),
            Window::Popup(popup) => {
                let parent = self.ui.window_id()
                    .surface()
                    .expect("attempting to open a popup during Ui init");

                let pos = self.ui.mouse_pos();
                let anchor_rect = match popup.location {
                    popup::Location::Cursor if pos.is_some()  => {
                        let pos = pos.unwrap();

                        Rect::new(pos.x, pos.y, 1f32, 1f32)
                    }
                    popup::Location::WidgetBounds | popup::Location::Cursor =>
                        self.layout(),
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
                make_ui
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

    #[inline]
    pub fn as_init_ctx<'b: 'a>(&'b mut self) -> InitCtx<'a> {
        InitCtx {
            active: self.active,
            tree: self.tree,
            ui: self.ui
        }
    }
}

impl<'a> DrawCtx<'a> {
    #[inline]
    pub fn draw(&mut self, id: impl Into<Id>) {
        let next = id.into().0;
        let state = &mut self.tree.widgets[next];

        let active = self.active;
        let widget = Rc::clone(&state.widget);
        let layout = state.layout;

        self.active = next;
        widget.draw(self, layout);
        self.active = active;
    }
}

impl<'a> InitCtx<'a> {
    pub fn new_child<E: Element>(&mut self, el: E) -> TypedId<E>
        where E::Widget: AnyWidget
    {
        // Hack, so we can get a key from the SlotMap
        let child = self.tree.widgets.insert(
            WidgetState::new(Rc::new(null_widget::NullWidget), Box::new(null_widget::State))
        );

        let parent = self.active;
        self.active = child;

        let (widget, state) = el.make_widget(self);

        self.active = parent;

        let state = WidgetState::new(Rc::new(widget), Box::new(state));
        self.tree.widgets[child] = state;

        self.tree.child_to_parent.insert(child, parent);

        match self.tree.parent_to_children.entry(parent) {
            Some(entry) => entry.or_default().push(child),
            None => {
                // We've reached the root widget which has no parent.
                assert_eq!(parent, RawWidgetId::null());
            }
        }

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
        pub fn register_action(&mut self, action: Action) -> ActionId {
            let actions = self.tree.actions.entry(self.active)
                .unwrap()
                .or_default();

            let action = actions.insert(action);

            ActionId { widget: self.active, action }
        }

        #[inline]
        pub fn value_sender<T: Send + 'static>(&self) -> ValueSender<T> {
            ValueSender::new(
                self.active,
                self.ui.task_send.clone()
            )
        }

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
            self.ui.rt_handle.spawn(task)
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
            let tx = self.ui.task_send.clone();
            let id = self.active;
            let window_id = self.ui.window_id();
    
            self.ui.rt_handle.spawn(async move {
                let result = task.await;
                let result = TaskResult {
                    id,
                    data: Box::new(result)
                };

                if tx.send(result).is_err() {
                    eprintln!("Failed to send task result to window {:?} - it has already closed.", window_id);
                }
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
                self.active,
                self.ui.task_send.clone()
            );
    
            self.ui.rt_handle.spawn(create_future(sender))
        }

        pub(crate) fn load_asset(&self, source: impl Into<AssetSource>) {
            let sender = ValueSender::new(
                self.active,
                self.ui.task_send.clone()
            );

            let job = asset_loader::Job {
                sender,
                source: source.into()
            };

            asset_loader::load(job);
        }
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

impl<E: Element> TypedId<E> {
    #[inline]
    fn raw(&self) -> RawWidgetId {
        self.id.0
    }
}

impl<E: Element> Into<Id> for TypedId<E> {
    #[inline]
    fn into(self) -> Id {
        Id(self.raw())
    }
}

impl<E: Element> Clone for TypedId<E> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            message: self.message,
            data: PhantomData,
        }
    }
}

impl<E: Element> Copy for TypedId<E> { }

impl fmt::Debug for TaskResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TaskResult")
            .field("id", &self.id)
            .field("data", &self.data)
            .finish()
    }
}

// This is a hack which exists because you cannot reserve a key on a SlotMap but we
// need to know the key before the new widget has been constructed. So we put this Null
// widget in temporarily and then replace it with the actual widget once it has been
// constructed. We use ZSTs so no allocations occur. We cannot use SlotMap::insert_with_key
// because we are are passing the InitCtx down the tree.
mod null_widget {
    use super::*;

    pub struct Null;
    pub struct NullWidget;
    pub struct State;

    impl Element for Null {
        type Widget = NullWidget;
        type Message = ();

        fn make_widget(self, _ctx: &mut InitCtx) -> (
            Self::Widget,
            <Self::Widget as Widget>::State
        ) {
            panic!("Called into null widget. This is a bug...");
        }
    }

    impl Widget for NullWidget {
        type State = State;

        fn layout(_handle: StateHandle<Self::State>, _ctx: &mut LayoutCtx, _bounds: SizeConstraints) -> Size {
            panic!("Called into null widget. This is a bug...");
        }

        fn draw(_handle: StateHandle<Self::State>, _ctx: &mut DrawCtx, _layout: Rect) {
            panic!("Called into null widget. This is a bug...");
        }
    }
}
