
use std::hash::{Hash, Hasher};

use mibar_core::{ValueSender, window::WindowId};
use ahash::AHasher;

#[derive(Clone, Copy, Eq, PartialEq, Hash, Debug)]
pub(crate) struct SenderKey(u64);

impl SenderKey {
    #[inline]
    pub fn new<T: Send>(window_id: WindowId, sender: &ValueSender<T>) -> Self {
        let mut hasher = AHasher::default();
        window_id.hash(&mut hasher);
        sender.hash(&mut hasher);

        Self(hasher.finish())
    }
}
