use std::{
    thread,
    io,
    fs,
    path::PathBuf,
    hash::{Hash, Hasher},
    fmt::{self, Display, Formatter},
    sync::mpsc as std_mpsc
};

use lazy_static::lazy_static;
use tiny_skia::Pixmap;
use ahash::AHasher;
use nohash;

use crate::ui::ValueSender;

lazy_static! {
    static ref LOADER_THREAD: std_mpsc::Sender<Job> = {
        let (tx, rx) = std_mpsc::channel();
        thread::spawn(|| start_resource_thread(rx));

        tx
    };
}

pub type LoadResult = std::result::Result<Pixmap, AssetLoadError>;

pub struct Job {
    pub sender: ValueSender<LoadResult>,
    pub source: AssetSource
}

#[derive(Clone, Copy, PartialOrd, Eq, PartialEq, Debug)]
pub struct AssetId(u64);

#[derive(Hash, Debug)]
pub enum AssetSource {
    /// Only PNG format is supported currently.
    Image(AssetDataSource)
}

#[derive(Hash, Debug)]
pub enum AssetDataSource {
    Path(PathBuf),
    Bytes(Vec<u8>),
    StaticBytes(&'static [u8])
}

#[derive(Debug)]
pub enum AssetLoadError {
    Io(io::Error),
    DecodeError(String)
}

#[inline]
pub fn load(job: Job) {
    LOADER_THREAD.send(job)
        .expect("Resource loader thread has crashed. This is a bug...");
}

fn start_resource_thread(recv: std_mpsc::Receiver<Job>) {
    while let Ok(job) = recv.recv() {
        let result = match job.source {
            AssetSource::Image(src) => load_image(src) 
        };

        job.sender.send(result);
    }
}

#[inline]
fn load_image(src: AssetDataSource) -> Result<Pixmap, AssetLoadError> {
    match src {
        AssetDataSource::Path(path) => {
            let bytes = fs::read(path)?;

            Pixmap::decode_png(&bytes)
        }
        AssetDataSource::Bytes(bytes) =>
            Pixmap::decode_png(&bytes),
        AssetDataSource::StaticBytes(bytes) =>
            Pixmap::decode_png(bytes)
    }
    .map_err(|x| AssetLoadError::DecodeError(x.to_string()))
}

impl AssetId {
    pub fn new(src: &AssetSource) -> Self {
        let mut hasher = AHasher::default();
        src.hash(&mut hasher);

        Self(hasher.finish())
    }
}

impl Hash for AssetId {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u64(self.0);
    }
}

impl nohash::IsEnabled for AssetId { }

impl AssetSource {
    #[inline]
    pub fn data_source(&self) -> &AssetDataSource {
        match self {
            Self::Image(src) => &src
        }
    }
}

impl From<AssetDataSource> for AssetSource {
    #[inline]
    fn from(src: AssetDataSource) -> Self {
        AssetSource::Image(src)
    }
}

impl From<PathBuf> for AssetDataSource {
    #[inline]
    fn from(path: PathBuf) -> Self {
        Self::Path(path)
    }
}

impl From<&str> for AssetDataSource {
    #[inline]
    fn from(path: &str) -> Self {
        Self::Path(PathBuf::from(path))
    }
}

impl From<&'static [u8]> for AssetDataSource {
    #[inline]
    fn from(bytes: &'static [u8]) -> Self {
        Self::StaticBytes(bytes)
    }
}

impl From<io::Error> for AssetLoadError {
    #[inline]
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}

impl Display for AssetLoadError {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            AssetLoadError::Io(err) => Display::fmt(err, f),
            AssetLoadError::DecodeError(err) => f.write_str(err)
        }
    }
}
