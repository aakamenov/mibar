pub mod workspaces;
pub mod date_time;
pub mod cpu;
pub mod ram;
pub mod battery;
pub mod volume;
pub mod keyboard_layout;
pub mod sys_info;
pub mod hyprland;
mod system_monitor;

use std::ops::Deref;

/// Pointer to a `static` variable.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) struct StaticPtr<T>(*const T);

impl<T> StaticPtr<T> {
    #[inline]
    pub fn new(item: &T) -> Self {
        Self(item)
    }
}

impl<T> Deref for StaticPtr<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        unsafe { &(*self.0) }
    }
}

unsafe impl<T> Sync for StaticPtr<T> { }
unsafe impl<T> Send for StaticPtr<T> { }
