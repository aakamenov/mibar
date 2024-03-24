use std::{
    thread,
    io,
    path::PathBuf,
    hash::{Hash, Hasher},
    fmt::{self, Display, Formatter},
    collections::hash_map::Entry,
    sync::{mpsc::{Sender, Receiver, channel}, Arc, Weak}
};

use once_cell::sync::Lazy;
use tiny_skia::{Pixmap as SkiaPixmap, IntSize};
use ahash::{AHasher, HashMapExt};
use nohash::IntMap;
use rgb::FromSlice;

#[cfg(feature = "svg")]
use resvg::{self, usvg::{Tree, Options}};

use crate::ui::ValueSender;

static LOADER_THREAD: Lazy<Sender<Job>> = Lazy::new(|| {
    let (tx, rx) = channel();
    thread::spawn(|| start_thread(rx));

    tx
});

pub mod png {
    use super::*;

    #[derive(Clone, Hash, Debug)]
    pub enum Data {
        Path(PathBuf),
        Bytes(Arc<[u8]>),
        StaticBytes(&'static [u8]),
        Rgba {
            width: u32,
            height: u32,
            pixels: Vec<u8>
        }
    }

    impl From<PathBuf> for Data {
        #[inline]
        fn from(path: PathBuf) -> Self {
            Self::Path(path)
        }
    }

    impl From<&str> for Data {
        #[inline]
        fn from(path: &str) -> Self {
            Self::Path(PathBuf::from(path))
        }
    }

    impl From<&'static [u8]> for Data {
        #[inline]
        fn from(bytes: &'static [u8]) -> Self {
            Self::StaticBytes(bytes)
        }
    }

    impl From<Vec<u8>> for Data {
        #[inline]
        fn from(bytes: Vec<u8>) -> Self {
            Self::Bytes(bytes.into())
        }
    }
}

#[cfg(feature = "svg")]
pub mod svg {
    use super::*;

    #[derive(Clone, Hash, Debug)]
    pub enum Data {
        Path(PathBuf),
        Bytes(Arc<[u8]>),
        StaticBytes(&'static [u8]),
    }

    impl From<PathBuf> for Data {
        #[inline]
        fn from(path: PathBuf) -> Self {
            Self::Path(path)
        }
    }

    impl From<&str> for Data {
        #[inline]
        fn from(path: &str) -> Self {
            Self::Path(PathBuf::from(path))
        }
    }

    impl From<&'static [u8]> for Data {
        #[inline]
        fn from(bytes: &'static [u8]) -> Self {
            Self::StaticBytes(bytes)
        }
    }

    impl From<Vec<u8>> for Data {
        #[inline]
        fn from(bytes: Vec<u8>) -> Self {
            Self::Bytes(bytes.into())
        }
    }
}

pub struct Job {
    pub sender: ValueSender<Result<Pixmap, Error>>,
    pub request: Request,
    pub resize: Option<Resize>
}

#[derive(Clone, Debug)]
pub enum Request {
    Png(png::Data),
    #[cfg(feature = "svg")]
    Svg {
        data: svg::Data,
        scale: f32
    }
} 

#[derive(Clone, Debug)]
pub struct Pixmap {
    pixels: Arc<[u8]>,
    size: (u32, u32),
    logical_size: (u32, u32)
}
    
#[derive(Clone, Copy, Hash, PartialEq, Debug)]
pub struct Resize {
    pub width: u32,
    pub height: u32,
    pub filter: Filter
}

/// Resizing filter to use.
///
/// For a detailed explanation and comparison of the different filters, see
/// [this article](https://www.imagemagick.org/Usage/filter/).
#[derive(Clone, Copy, Hash, Eq, PartialEq, Debug)]
pub enum Filter {
    /// Uses [`Filter::Lanczos3`] if downscaling, [`Filter::Mitchell`]
    /// if upscaling or [`Filter::Catrom`] otherwise.
    Auto,
    /// Point resizing/nearest neighbor.
    ///
    /// This is the fastest method, but also has the lowest quality. It will
    /// produce block/aliased results.
    Point,
    /// Triangle (bilinear) resizing.
    ///
    /// A fast method that produces smooth results.
    Triangle,
    /// Catmull-Rom (bicubic) resizing.
    ///
    /// This is the default cubic filter in many image editing programs. It
    /// produces sharp results for both upscaling and downscaling.
    Catrom,
    /// Resize using the (bicubic) Mitchell-Netravali filter.
    ///
    /// This filter is similar to [Type::Catrom], but produces slightly
    /// smoother results, which can eliminate over-sharpening artifacts when
    /// upscaling.
    Mitchell,
    /// B-spline (bicubic) resizing.
    ///
    /// This filter produces smoother results than [Type::Catrom] and
    /// [Type::Mitchell]. It can appear a little blurry, but not as blurry as
    /// [Type::Gaussian].
    BSpline,
    /// Gaussian resizing.
    ///
    /// Uses a Gaussian function as a filter. This is a slow filter that produces
    /// very smooth results akin to a slight gaussian blur. Its main advantage
    /// is that it doesn't introduce ringing or aliasing artifacts.
    Gaussian,
    /// Resize using Sinc-windowed Sinc with radius of 3.
    ///
    /// A slow filter that produces sharp results, but can have ringing.
    /// Recommended for high-quality image resizing.
    Lanczos3,
}

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    DecodeError(String),
    InvalidSize
}

#[derive(Clone, Debug)]
struct PixmapWeak {
    pixels: Weak<[u8]>,
    size: (u32, u32),
    logical_size: (u32, u32)
}

#[inline]
pub fn load(job: Job) {
    LOADER_THREAD.send(job)
        .expect("Resource loader thread has crashed. This is a bug...");
}

fn start_thread(recv: Receiver<Job>) {
    let mut cache = IntMap::<u64, PixmapWeak>::new();

    while let Ok(job) = recv.recv() {
        let mut hasher = AHasher::default();

        match &job.request {
            Request::Png(data) => data.hash(&mut hasher),
            #[cfg(feature = "svg")]
            Request::Svg { data, scale } => {
                data.hash(&mut hasher);
                scale.to_bits().hash(&mut hasher);
            }
        }

        job.resize.hash(&mut hasher);

        let id = hasher.finish();

        let result = match cache.entry(id) {
            Entry::Occupied(mut entry) => {
                let pixmap = entry.get_mut();

                if let Some(pixels) = pixmap.pixels.upgrade() {
                    Ok(Pixmap {
                        pixels,
                        size: pixmap.size,
                        logical_size: pixmap.logical_size
                    })
                } else {
                    let result = make_pixmap(job.request, job.resize);

                    match result.as_ref() {
                        Ok(pixmap) => entry.insert(pixmap.as_weak()),
                        Err(_) => entry.remove()
                    };

                    result
                }
            }
            Entry::Vacant(entry) => {
                let result = make_pixmap(job.request, job.resize);

                if let Ok(pixmap) = result.as_ref() {
                    entry.insert(pixmap.as_weak());
                }

                result
            }
        };

        job.sender.send(result);
    }
}
    
#[inline]
fn make_pixmap(request: Request, resize: Option<Resize>) -> Result<Pixmap, Error> {
    let pixmap = match request {
        Request::Png(data) => Pixmap::from_png(data),
        #[cfg(feature = "svg")]
        Request::Svg { data, scale } => Pixmap::from_svg(data, scale)
    }?;

    if let Some(resize) = resize {
        pixmap.resize(resize).ok_or(Error::InvalidSize)
    } else {
        Ok(pixmap)
    }
}

impl Pixmap {
    pub fn from_png(data: png::Data) -> Result<Self, Error> {
        let pixmap = match data {
            png::Data::Path(path) => SkiaPixmap::load_png(path),
            png::Data::Bytes(bytes) => SkiaPixmap::decode_png(&bytes),
            png::Data::StaticBytes(bytes) => SkiaPixmap::decode_png(&bytes),
            png::Data::Rgba { width, height, pixels } => {
                let size = IntSize::from_wh(width, height)
                    .ok_or(Error::InvalidSize)?;

                let pixmap = SkiaPixmap::from_vec(pixels, size)
                    .ok_or(Error::InvalidSize)?;

                Ok(pixmap)

            }
        }.map_err(|x| Error::DecodeError(x.to_string()))?;

        let width = pixmap.width();
        let height = pixmap.height();
        let pixels = Arc::from(pixmap.take());
        
        Ok(Self {
            pixels,
            size: (width, height),
            logical_size: (width, height)
        })
    }

    #[cfg(feature = "svg")]
    pub fn from_svg(data: svg::Data, scale: f32) -> Result<Self, Error> {
        let options = Options::default();
        let tree = match data {
            svg::Data::Path(path) => {
                let bytes = std::fs::read(path)?;

                Tree::from_data(&bytes, &options)
            }
            svg::Data::Bytes(bytes) => Tree::from_data(&bytes, &options),
            svg::Data::StaticBytes(bytes) => Tree::from_data(&bytes, &options)
        }.map_err(|x| Error::DecodeError(x.to_string()))?;

        let size = tree.size().to_int_size();

        let transform = tiny_skia::Transform::from_scale(scale, scale);
        let pixmap_size = size.scale_by(scale)
            .ok_or(Error::InvalidSize)?
            .dimensions();

        let mut pixmap = SkiaPixmap::new(
            pixmap_size.0,
            pixmap_size.1
        ).ok_or(Error::InvalidSize)?;

        resvg::render(&tree, transform, &mut pixmap.as_mut());

        assert_eq!(pixmap.data().len() % 4, 0);

        for pixel in pixmap.data_mut().chunks_exact_mut(4) {
            unsafe {
                std::ptr::swap_nonoverlapping(
                    pixel.get_unchecked_mut(0) as *mut u8,
                    pixel.get_unchecked_mut(2) as *mut u8,
                    1
                );
            }
        }

        let pixels = Arc::from(pixmap.take());

        Ok(Self {
            pixels,
            size: pixmap_size,
            logical_size: (size.width(), size.height())
        })
    }

    #[inline]
    pub fn physical_size(&self) -> (u32, u32) {
        self.size
    }

    #[inline]
    pub fn logical_size(&self) -> (u32, u32) {
        self.logical_size
    }

    /// Byteorder: RGBA
    #[inline]
    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }

    #[inline]
    pub fn scale(&self) -> f32 {
        self.size.0 as f32 / self.logical_size.0 as f32
    }

    pub fn resize(&self, resize: Resize) -> Option<Self> {
        if self.logical_size.0 == resize.width && self.logical_size.1 == resize.height {
            return Some(self.clone());
        }

        let width = resize.width as usize;
        let height = resize.height as usize;
        let mut buf = vec![0u8; (width * height * 4) as usize];

        let filter = match resize.filter {
            Filter::Auto =>
                if self.size.0 > resize.width && self.size.1 > resize.height {
                    resize::Type::Lanczos3
                } else if self.size.0 < resize.width && self.size.1 < resize.height {
                    resize::Type::Mitchell
                } else {
                    resize::Type::Catrom
                }
            Filter::Point => resize::Type::Point,
            Filter::Triangle => resize::Type::Triangle,
            Filter::Catrom => resize::Type::Catrom,
            Filter::Mitchell => resize::Type::Mitchell,
            Filter::BSpline => resize::Type::BSpline,
            Filter::Gaussian => resize::Type::Gaussian,
            Filter::Lanczos3 => resize::Type::Lanczos3
        };

        let mut resizer = resize::new(
            self.size.0 as usize,
            self.size.1 as usize,
            width,
            height,
            resize::Pixel::RGBA8,
            filter
        ).ok()?;

        resizer.resize(self.pixels.as_rgba(), buf.as_rgba_mut()).ok()?;

        Some(Self {
            size: (resize.width, resize.height),
            logical_size: (resize.width, resize.height),
            pixels: Arc::from(buf)
        })
    }

    #[inline]
    fn as_weak(&self) -> PixmapWeak {
        PixmapWeak {
            pixels: Arc::downgrade(&self.pixels),
            size: self.size,
            logical_size: self.logical_size
        }
    }
}

impl From<io::Error> for Error {
    #[inline]
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}

impl Display for Error {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(err) => Display::fmt(err, f),
            Self::DecodeError(err) => f.write_str(err),
            Self::InvalidSize => f.write_str("Invalid size for RGBA pixels provided.")
        }
    }
}
