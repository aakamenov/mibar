use std::{
    fmt,
    any::Any,
    future::Future,
    marker::PhantomData,
    collections::VecDeque,
    borrow::Borrow,
    hash::{Hash, Hasher}
};

use tiny_skia::PixmapMut;
use tokio::{runtime, task::JoinHandle, sync::mpsc::UnboundedSender};
use smithay_client_toolkit::reexports::calloop::channel::Sender;
use slotmap::{SlotMap, SecondaryMap, Key, new_key_type};
use smallvec::SmallVec;

use crate::{
    geometry::{Rect, Size, Point},
    widget::{
        Element, Widget, AnyWidget, SizeConstraints
    },
    theme::Theme,
    draw::TextInfo,
    renderer::Renderer,
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

pub struct TypedId<E: Element> {
    id: Id,
    message: fn(
        &mut <E::Widget as Widget>::State,
        &mut UpdateCtx,
        E::Message
    ),
    data: PhantomData<E>
}

#[derive(Eq, PartialEq, Hash, Debug)]
pub struct Id(RawWidgetId);

pub struct LayoutCtx<'a> {
    ui: &'a mut UiCtx
}

pub struct DrawCtx<'a> {
    ui: &'a mut UiCtx,
    layout: Rect
}

pub struct UpdateCtx<'a> {
    ui: &'a mut UiCtx,
    current: RawWidgetId
}

pub struct InitCtx<'a> {
    pub(crate) ui: &'a mut UiCtx,
    current: RawWidgetId
}

#[derive(Debug)]
pub enum Event {
    Mouse(MouseEvent)
}

#[derive(Debug)]
pub(crate) enum UiEvent {
    Window(WindowEvent),
    Task(TaskResult)
}

pub(crate) struct Ui {
    pub(crate) ctx: UiCtx,
    root: Id,
    size: Size
}

pub(crate) struct UiCtx {
    pub(crate) renderer: Renderer,
    // Each Ui keeps a local copy of the current Theme. Whenever the theme
    // is mutated, the Ui sends a request to the client which then propagates
    // the changes to all the other windows. This may be more expensive than
    // using a mutex but changing the theme in practice happens rarely (if ever)
    // as opposed to synchronizing access every time we want to read it which
    // occurs multiple times per UI re-draw.
    theme: Theme,
    mouse_pos: Option<Point>,
    widgets: SlotMap<RawWidgetId, WidgetState>,
    needs_redraw: bool,
    needs_layout: bool,
    parent_to_children: SecondaryMap<RawWidgetId, SmallVec<[RawWidgetId; 4]>>,
    child_to_parent: SecondaryMap<RawWidgetId, RawWidgetId>,
    rt_handle: runtime::Handle,
    task_send: Sender<TaskResult>,
    client_send: UnboundedSender<UiRequest>,
    window_id: WindowId
}

new_key_type! {
    /// Internal id that unlike `Id` or `TypedId` (which wrap around it and disable Copy/Clone)
    /// doesn't track widget lifetimes. Should be used with great care!!!
    pub(crate) struct RawWidgetId;
}

struct WidgetState {
    widget: Box<dyn AnyWidget>,
    state: Box<dyn Any>,
    layout: Rect
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
        let mut ctx = UiCtx {
            renderer: Renderer::new(),
            theme,
            mouse_pos: None,
            widgets: SlotMap::with_capacity_and_key(20),
            needs_redraw: false,
            needs_layout: false,
            parent_to_children: SecondaryMap::new(),
            child_to_parent: SecondaryMap::new(),
            rt_handle,
            task_send,
            client_send,
            window_id
        };

        let mut init_ctx = InitCtx {
            current: RawWidgetId::null(),
            ui: &mut ctx
        };

        let root = init_ctx.new_child(root);

        Self {
            root: root.into(),
            ctx,
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
            ui: &mut self.ctx,
            current: self.root.0
        };

        ctx.event(&self.root, &event);
    }

    pub fn task_result(&mut self, result: TaskResult) {
        // Widget might have been removed while the task was executing.
        let Some(state) = self.ctx.widgets.get_mut(result.id) else {
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
            self.layout_impl(SizeConstraints::tight(self.size));
        }

        let mut ctx = DrawCtx {
            ui: &mut self.ctx,
            layout: Rect::default()
        };

        ctx.draw(&self.root);

        self.ctx.needs_redraw = false;
        self.ctx.needs_layout = false;

        self.ctx.renderer.render(pixmap);
    }

    pub fn destroy(mut self) {
        self.ctx.dealloc(self.root);
    }

    #[inline]
    pub fn needs_redraw(&self) -> bool {
        self.ctx.needs_redraw
    }

    #[inline]
    pub fn set_scale_factor(&mut self, scale_factor: f32) {
        if self.ctx.renderer.scale_factor() != scale_factor {
            self.ctx.renderer.set_scale_factor(scale_factor);
            self.ctx.needs_redraw = true;
        }
    }

    fn layout_impl(&mut self, constraints: SizeConstraints) -> Size {
        let mut ctx = LayoutCtx {
            ui: &mut self.ctx
        };

        let size = ctx.layout(&self.root, constraints);

        // Translate all widget positions from parent local space
        // to window space. This feels like a giant hack but enables us to have
        // a separate draw step as otherwise the only way to translate
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
            let children = self.ctx.parent_to_children.entry(current).unwrap().or_default();
            queue.extend(children.iter());

            let offset = self.ctx.widgets[current].layout.origin();
            
            for child in children {
                let state = self.ctx.widgets.get_mut(*child).unwrap();
    
                state.layout.x += offset.x;
                state.layout.y += offset.y;
            }
        }

        size
    }
}

impl UiCtx {
    #[inline]
    pub fn window_id(&self) -> WindowId {
        self.window_id
    }

    fn dealloc(&mut self, id: Id) {
        let widget = self.widgets.remove(id.0).unwrap();

        let parent = self.child_to_parent.remove(id.0).unwrap();
        let children = self.parent_to_children.get_mut(parent).unwrap();
        let index = children.iter().position(|x| *x == id.0).unwrap();

        // Can't use swap_remove() because order is important
        children.remove(index);

        let children = self.parent_to_children
            .remove(id.0)
            .unwrap_or_default();

        for child in children {
            self.dealloc_impl(child);
        }

        // We call destroy after we've deallocated the children as
        // they could be relying on the parent being alive during their
        // destroy() call (as is the case with StateHandle<T> in the State widget).
        widget.widget.destroy(widget.state);
    }

    fn dealloc_impl(&mut self, id: RawWidgetId) {
        let widget = self.widgets.remove(id).unwrap();

        self.child_to_parent.remove(id).unwrap();

        let children = self.parent_to_children
            .remove(id)
            .unwrap_or_default();

        for child in children {
            self.dealloc_impl(child);
        }

        // We call destroy after we've deallocated the children as
        // they could be relying on the parent being alive during their
        // destroy() call (as is the case with StateHandle<T> in the State widget).
        widget.widget.destroy(widget.state);
    }
}

impl<'a> LayoutCtx<'a> {
    #[inline]
    pub fn layout(&mut self, id: impl Borrow<Id>, bounds: SizeConstraints) -> Size {
        let state = self.ui.widgets.get_mut(id.borrow().0)
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
        self.ui.renderer.text_renderer.measure(info, size)
    }

    #[inline]
    pub fn position(
        &mut self,
        id: impl Borrow<Id>,
        func: impl FnOnce(&mut Rect)
    ) -> Rect {
        let state = self.ui.widgets.get_mut(id.borrow().0).unwrap();
        func(&mut state.layout);

        state.layout
    }
}

impl<'a> UpdateCtx<'a> {
    #[inline]
    pub fn event(&mut self, id: impl Borrow<Id>, event: &Event) {
        let id = id.borrow().0;
        let state = self.ui.widgets.get_mut(id)
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
        self.ui.widgets[self.current].layout
    }

    #[inline]
    pub fn message<E: Element>(&mut self, id: &TypedId<E>, msg: E::Message) {
        let state = self.ui.widgets.get_mut(id.raw())
            .unwrap() as *mut WidgetState;

        let prev = self.current;
        self.current = id.raw();

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
                let parent = self.window_id()
                    .surface()
                    .expect("attempting to open a popup during Ui init");

                let pos = self.mouse_pos();
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
            current: self.current,
            ui: self.ui
        }
    }
}

impl<'a> DrawCtx<'a> {
    #[inline(always)]
    pub fn renderer(&mut self) -> &mut Renderer {
        &mut self.ui.renderer
    }

    #[inline]
    pub fn draw(&mut self, id: impl Borrow<Id>) {
        let state = self.ui.widgets.get_mut(id.borrow().0)
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
    pub fn layout(&self) -> Rect {
        self.layout
    }
}

impl<'a> InitCtx<'a> {
    pub fn new_child<E: Element>(&mut self, el: E) -> TypedId<E>
        where E::Widget: AnyWidget
    {
        // Hack, so we can get a key from the SlotMap
        let child = self.ui.widgets.insert(
            WidgetState::new(Box::new(null_widget::NullWidget), Box::new(null_widget::State))
        );

        let parent = self.current;
        self.current = child;

        let (widget, state) = el.make_widget(self);

        self.current = parent;

        let state = WidgetState::new(Box::new(widget), Box::new(state));
        self.ui.widgets[child] = state;

        self.ui.child_to_parent.insert(child, parent);

        match self.ui.parent_to_children.entry(parent) {
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
        pub fn runtime_handle(&self) -> &runtime::Handle {
            &self.ui.rt_handle
        }

        #[inline]
        pub fn value_sender<T: Send + 'static>(&self) -> ValueSender<T> {
            ValueSender::new(
                self.current,
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
            let id = self.current;
    
            self.ui.rt_handle.spawn(async move {
                let result = task.await;
                let result = TaskResult {
                    id,
                    data: Box::new(result)
                };

                tx.send(result).unwrap();
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
                self.ui.task_send.clone()
            );
    
            self.ui.rt_handle.spawn(create_future(sender))
        }

        pub fn theme_mut(&mut self, change: impl FnOnce(&mut Theme)) {
            change(&mut self.ui.theme);
            self.ui.needs_redraw = true;

            self.ui.client_send.send(
                UiRequest {
                    id: self.ui.window_id,
                    action: WindowAction::ThemeChanged(self.ui.theme.clone())
                }
            ).unwrap();
        }

        #[inline]
        pub fn window_id(&self) -> WindowId {
            self.ui.window_id()
        }

        pub(crate) fn load_asset(&self, source: impl Into<AssetSource>) {
            let sender = ValueSender::new(
                self.current,
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

impl_context_method! {
    InitCtx<'_>,
    LayoutCtx<'_>,
    UpdateCtx<'_>,
    DrawCtx<'_>,
    {
        /// `None` means the mouse is currently outside the window.
        #[inline]
        pub fn mouse_pos(&self) -> Option<Point> {
            self.ui.mouse_pos
        }

        #[inline]
        pub fn theme(&self) -> &Theme {
            &self.ui.theme
        }
    }
}

impl_context_method! {
    UpdateCtx<'_>,
    DrawCtx<'_>,
    {
        #[inline]
        pub fn is_hovered(&self) -> bool {
            if let Some(pos) = self.ui.mouse_pos {
                self.layout().contains(pos)
            } else {
                false
            }
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
    fn new(id: RawWidgetId, sender: Sender<TaskResult>) -> Self {
        Self {
            id,
            sender,
            phantom: PhantomData
        }
    }

    #[inline]
    pub fn send(&self, value: T) {
        let result = TaskResult {
            id: self.id,
            data: Box::new(value)
        };

        self.sender.send(result).unwrap()
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

        fn layout(_state: &mut Self::State, _ctx: &mut LayoutCtx, _bounds: SizeConstraints) -> Size {
            panic!("Called into null widget. This is a bug...");
        }

        fn draw(_state: &mut Self::State, _ctx: &mut DrawCtx) {
            panic!("Called into null widget. This is a bug...");
        }
    }
}
