use std::{
    mem,
    num::NonZeroUsize,
    collections::{HashMap, HashSet, hash_map::Entry},
    hash::{Hash, Hasher}
};

use ahash::AHasher;
use lru::LruCache;
use tiny_skia::{Pixmap, PixmapRef, Color, ColorU8};
use cosmic_text::{
    FontSystem, Buffer, Attrs, Metrics, Shaping,
    SwashCache, SwashContent, Placement
};

use crate::{geometry::Size, theme::Font};

const GLYPH_CACHE_SIZE: usize = 64;
const TRIM_ROUNDS: u8 = 3;

pub struct Renderer {
    font_system: FontSystem,
    cache: HashMap<CacheKey, CachedText>,
    recently_used: HashSet<CacheKey>,
    trim_rounds: u8,
    glyph_cache: GlyphCache
}

#[derive(Clone, Debug)]
pub struct TextInfo {
    pub text: String,
    pub size: f32,
    pub line_height: LineHeight,
    pub font: Font
}

// We use the same approach as Iced here. The rationale being
// that multiplying the text size by 1.2 will work for most fonts.
// So this is what is used as a default (LineHeight::Relative(1.2)).
// Apparently this is what web browsers tend to do as well.
// Otherwise, it can be set by the user if needed.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum LineHeight {
    /// A scale that the size of the text is multiplied by.
    Relative(f32),
    /// An absolute height in logical pixels.
    Absolute(f32)
}

#[derive(Clone, Copy, Hash, PartialEq, Eq, Debug)]
pub(super) struct CacheKey(u64);

struct GlyphCache {
    cache: LruCache<CachedGlyphKey, CachedGlyph>
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
struct CachedGlyphKey {
    swash_key: cosmic_text::CacheKey,
    /// RGB color
    color: [u8; 3]
}

#[derive(Debug)]
struct CachedGlyph {
    image: Pixmap,
    placement: Placement
}

struct CachedText {
    texture: Option<(Color, Pixmap)>,
    buffer: Buffer,
    requested_size: Size,
    computed_size: Size
}

impl Renderer {
    pub fn new() -> Self {
        Self {
            font_system: FontSystem::new(),
            cache: HashMap::new(),
            recently_used: HashSet::new(),
            trim_rounds: 0,
            glyph_cache: GlyphCache {
                cache: LruCache::new(NonZeroUsize::new(GLYPH_CACHE_SIZE).unwrap())
            }
        }
    }

    #[inline]
    pub fn measure(&mut self, info: &TextInfo, size: Size) -> Size {
        let key = CacheKey::new(info);
        match self.cache.entry(key) {
            Entry::Occupied(mut entry) => {
                let text = entry.get_mut();
                text.layout(&mut self.font_system, size);

                text.computed_size
            },
            Entry::Vacant(entry) => {
                let text = CachedText::new(info, &mut self.font_system, size);
                let size = text.computed_size;

                entry.insert(text);

                size
            }
        }
    }

    pub(super) fn trim(&mut self) {
        self.trim_rounds += 1;

        if self.trim_rounds == TRIM_ROUNDS {
            self.trim_rounds = 0;
            self.cache.retain(|key, _| self.recently_used.contains(key));
            self.recently_used.clear();
        }
    }

    pub(super) fn ensure_is_cached(&mut self, info: &TextInfo, size: Size) -> CacheKey {
        let key = CacheKey::new(info);
        if let Entry::Vacant(entry) = self.cache.entry(key) {
            let text = CachedText::new(info, &mut self.font_system, size);
            entry.insert(text);
        }

        key
    }

    pub(super) fn get_texture(&mut self, key: CacheKey, color: Color) -> PixmapRef {
        let text = self.cache.get_mut(&key).expect("must call ensure_is_cached() first");
        self.recently_used.insert(key);

        // Shitty hack to appease the borrow checker.
        if let Some(same_color) = text.texture.as_ref().and_then(|x| Some(x.0 == color)) {
            if same_color {
                return text.texture.as_ref().unwrap().1.as_ref();
            }
        }

        let mut cache = SwashCache::new();

        let mut pixmap = Pixmap::new(
            text.computed_size.width.ceil() as u32,
            text.computed_size.height.ceil() as u32
        ).unwrap();

        for run in text.buffer.layout_runs() {
            for glyph in run.glyphs {
                // TODO: Get scale factor from compositor.
                const SCALE_FACTOR: f32 = 1f32;
                let phys_glyph = glyph.physical((0., 0.), SCALE_FACTOR);

                let key = CachedGlyphKey::new(phys_glyph.cache_key, color);
                if let Some(glyph) = self.glyph_cache.get_or_create(
                    &mut self.font_system,
                    &mut cache,
                    key
                ) {
                    pixmap.draw_pixmap(
                        phys_glyph.x + glyph.placement.left,
                        phys_glyph.y - glyph.placement.top +
                            (run.line_y * SCALE_FACTOR).round() as i32,
                        glyph.image.as_ref(),
                        &tiny_skia::PixmapPaint::default(),
                        tiny_skia::Transform::identity(),
                        None
                    );
                }
            }
        }

        text.texture = Some((color, pixmap));

        text.texture.as_ref().unwrap().1.as_ref()
    }
}

impl GlyphCache {
    fn get_or_create(
        &mut self,
        font_system: &mut FontSystem,
        cache: &mut SwashCache,
        key: CachedGlyphKey
    ) -> Option<&CachedGlyph> {
        struct NoGlyphImageErr;

        let glyph = self.cache.try_get_or_insert(key, || {
            let Some(image) = cache.get_image_uncached(
                font_system,
                key.swash_key
            ) else {
                return Err(NoGlyphImageErr);
            };

            let placement = image.placement;

            let mut pixmap = Pixmap::new(placement.width, placement.height)
                .ok_or(NoGlyphImageErr)?;

            let pixels = pixmap.pixels_mut();

            match image.content {
                SwashContent::Color => {
                    let mut i = 0;
    
                    for _ in 0..placement.height {
                        for _ in 0..placement.width {
                            let color = ColorU8::from_rgba(
                                image.data[i],
                                image.data[i + 1],
                                image.data[i + 2],
                                image.data[i + 3]
                            ).premultiply();
    
                            pixels[i >> 2] = color;
                            i += 4;
                        }
                    }
                }
                SwashContent::Mask => {
                    let r = key.color[0];
                    let g = key.color[1];
                    let b = key.color[2];
    
                    let mut i = 0;
    
                    for _ in 0..placement.height {
                        for _ in 0..placement.width {
                            let color = ColorU8::from_rgba(
                                r,
                                g,
                                b,
                                image.data[i]
                            ).premultiply();
                            
                            pixels[i] = color;
    
                            i += 1;
                        }
                    }
                }
                SwashContent::SubpixelMask => { }
            }

            Ok(CachedGlyph {
                image: pixmap,
                placement
            })
        });

        glyph.ok()
    }
}

impl CachedGlyphKey {
    #[inline]
    fn new(swash_key: cosmic_text::CacheKey, color: Color) -> Self {
        let color = color.premultiply().to_color_u8();
        let color = [color.red(), color.green(), color.blue()];

        Self { swash_key, color }
    }
}

impl TextInfo {
    #[inline]
    pub fn new(text: impl Into<String>, size: f32) -> Self {
        Self {
            text: text.into(),
            size,
            line_height: LineHeight::default(),
            font: Font::default()
        }
    }

    #[inline]
    pub fn with_font(mut self, font: Font) -> Self {
        self.font = font;

        self
    }

    #[inline]
    pub fn with_line_height(mut self, height: LineHeight) -> Self {
        self.line_height = height;

        self
    }
}

impl CachedText {
    fn new(info: &TextInfo, font_system: &mut FontSystem, size: Size) -> Self {
        let line_height = info.line_height.to_absolute(info.size);
        let metrics = Metrics {
            font_size: info.size,
            line_height
        };

        let attrs = Attrs {
            color_opt: None,
            family: info.font.family,
            stretch: info.font.stretch,
            style: info.font.style,
            weight: info.font.weight,
            metadata: 0
        };

        let mut buffer = Buffer::new_empty(metrics);
        buffer.set_size(font_system, size.width, size.height);
        buffer.set_text(font_system, &info.text, attrs, Shaping::Basic);

        let mut text = CachedText {
            texture: None,
            buffer,
            requested_size: size,
            computed_size: Size::ZERO
        };
        text.compute_size();

        text
    }

    #[inline]
    fn layout(&mut self, font_system: &mut FontSystem, size: Size) {
        if self.requested_size == size {
            return;
        }

        self.buffer.set_size(font_system, size.width, size.height);
        self.requested_size = size;

        let prev = self.computed_size;
        self.compute_size();

        if prev != self.computed_size {
            self.texture = None;
        }
    }

    #[inline]
    fn compute_size(&mut self) {
        let mut lines = 0;
        let mut width = 0f32;

        for run in self.buffer.layout_runs() {
            lines += 1;
            width = run.line_w.max(width);
        }

        let line_height = self.buffer.metrics().line_height;
        self.computed_size = Size::new(width, lines as f32 * line_height);
    }
}

impl CacheKey {
    fn new(info: &TextInfo) -> Self {
        let ref mut hasher = AHasher::default();
        info.text.hash(hasher);
        info.size.to_bits().hash(hasher);
        info.line_height.hash(hasher);
        info.font.hash(hasher);

        Self(hasher.finish())
    }
}

impl LineHeight {
    pub fn to_absolute(self, text_size: f32) -> f32 {
        match self {
            Self::Relative(scale) => scale * text_size,
            Self::Absolute(height) => height
        }
    }
}

impl Hash for LineHeight {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        let variant = mem::discriminant(self);
        
        match self {
            LineHeight::Relative(scale) => 
                (variant, scale.to_bits()).hash(state),
            LineHeight::Absolute(height) =>
                (variant, height.to_bits()).hash(state)
        }
    }
}

impl Default for LineHeight {
    fn default() -> Self {
        Self::Relative(1.2)
    }
}
