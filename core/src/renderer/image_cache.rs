use std::collections::hash_map::Entry as MapEntry;

use nohash::IntMap;
use tiny_skia::{Pixmap, PixmapRef};

use crate::{asset_loader::AssetId, geometry::Size};

pub struct ImageCache {
    cache: IntMap<AssetId, Entry>
}

#[derive(Debug)]
struct Entry {
    ref_count: u8,
    /// If `None`, the image is being loaded.
    image: Option<Pixmap>
}

impl ImageCache {
    pub fn new() -> Self {
        Self { cache: IntMap::default() }
    }

    #[inline]
    pub fn get(&self, id: AssetId) -> Option<PixmapRef> {
        let Some(image) = self.cache.get(&id).map(|x| &x.image) else {
            return None;
        };

        if let Some(image) = image.as_ref() {
            return Some(image.as_ref());
        } else {
            None
        }
    }

    #[inline]
    pub fn size(&self, id: AssetId) -> Option<Size> {
        let entry = self.cache.get(&id)?;

        entry.image.as_ref()
            .map(|x| Size::new(x.width() as f32, x.height() as f32))
    }

    #[inline]
    pub fn allocate(&mut self, id: AssetId, image: Pixmap) {
        match self.cache.entry(id) {
            MapEntry::Occupied(mut entry) => {
                let entry = entry.get_mut();
                assert!(entry.image.replace(image).is_none());
            }
            MapEntry::Vacant(_) =>
                panic!("Trying to allocate an image without having increased ref count before! This is a bug...")
        };
    }

    /// Returns `true` if the cache already has an entry for the given `id`.
    /// Otherwise, the image widget should try to load the asset first.
    #[inline]
    pub fn increase_ref_count(&mut self, id: AssetId) -> bool {
        match self.cache.entry(id) {
            MapEntry::Occupied(mut entry) => {
                entry.get_mut().ref_count += 1;

                true
            }
            MapEntry::Vacant(entry) => {
                entry.insert(Entry { ref_count: 1, image: None });

                false
            }
        }
    }

    #[inline]
    pub fn decrease_ref_count(&mut self, id: AssetId) {
        if let MapEntry::Occupied(mut entry) = self.cache.entry(id) {
            let value = entry.get_mut();
            value.ref_count -= 1;

            if value.ref_count == 0 {
                entry.remove();
            }
        }
    }
}
