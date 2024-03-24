mod icon;

use std::any::Any;

use mibar_core::{
    widget::{SizeConstraints, Element, Widget},
    image::{Resize, Filter, png},
    Size, Rect, Context, DrawCtx, LayoutCtx,
    TypedId, Id, StateHandle
};
use smallvec::SmallVec;

use super::{subscribe, Event as TrayEvent, SniId, SubscriptionToken};
use icon::Icon;

const SPACING: f32 = 4f32;

pub struct Tray {
    pub icon_size: u32
}

#[derive(Default)]
pub struct TrayWidget;

pub struct State {
    icon_size: u32,
    icons: SmallVec<[Child; 8]>,
    _token: SubscriptionToken,
}

struct Child {
    icon_widget: TypedId<Icon>,
    id: SniId 
}

impl Tray {
    pub fn new(icon_size: u32) -> Self {
        Self { icon_size }
    }
}

impl Element for Tray {
    type Widget = TrayWidget;

    fn make_state(self, id: Id, ctx: &mut Context) -> <Self::Widget as Widget>::State {
        let token = subscribe(
            ctx.ui.runtime_handle(),
            ctx.ui.window_id(),
            ctx.ui.value_sender(id)
        );

        State {
            icon_size: self.icon_size,
            icons: SmallVec::new(),
            _token: token
        }
    }
}

impl Widget for TrayWidget {
    type State = State;

    fn layout(handle: StateHandle<Self::State>, ctx: &mut LayoutCtx, bounds: SizeConstraints) -> Size {
        let len = ctx.tree[handle].icons.len();
        let mut width = 0f32;
        let mut height = 0f32;
        let mut available = bounds.max.width;

        for i in 0..len {
            let bounds = SizeConstraints::new(
                Size::ZERO,
                Size::new(available, bounds.max.height)
            );

            let icon = ctx.tree[handle].icons[i].icon_widget;
            let size = ctx.layout(icon, bounds);
            ctx.position(icon, |rect| rect.x += width);

            let total = size.width + SPACING;
            width += total;
            height = height.max(size.height);

            available -= total;
        }

        width = (width - SPACING).max(0f32);
        
        bounds.constrain(Size::new(width, height))
    }

    fn task_result(handle: StateHandle<Self::State>, ctx: &mut Context, data: Box<dyn Any>) {
        let event = data.downcast::<TrayEvent>().unwrap();
        match *event {
            TrayEvent::New { id, mut icon } => {
                let state = &mut ctx.tree[handle];

                let resize = Resize {
                    width: state.icon_size,
                    height: state.icon_size,
                    filter: Filter::Auto,
                };

                #[cfg(target_endian = "little")]
                if let Some(png::Data::Rgba { pixels, .. }) = icon.as_mut() {
                    for chunk in pixels.chunks_exact_mut(4) {
                        let bytes = unsafe { *(chunk.as_ptr() as *const [u8; 4]) };
                        chunk[0] = bytes[3];
                        chunk[1] = bytes[2];
                        chunk[2] = bytes[1];
                        chunk[3] = bytes[0];
                    }
                }

                let icon_widget = ctx.new_child(handle, Icon { icon, resize });
                ctx.tree[handle].icons.push(Child { id, icon_widget });

            }
            TrayEvent::Remove(id) => {
                let state = &mut ctx.tree[handle];

                if let Some(index) = state.icons.iter().position(|x| x.id == id) {
                    let child = state.icons.remove(index);
                    ctx.event_queue.destroy(child.icon_widget);
                }

            },
            TrayEvent::Crash => {},
        }

    }

    fn draw(handle: StateHandle<Self::State>, ctx: &mut DrawCtx, _layout: Rect) {
        let len = ctx.tree[handle].icons.len();

        for i in 0..len {
            ctx.draw(ctx.tree[handle].icons[i].icon_widget);
        }
    }
}
