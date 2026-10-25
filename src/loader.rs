use std::collections::{HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Condvar, Mutex, OnceLock};
use std::time::Duration;

use eframe::egui::{self, ColorImage, TextureFilter, TextureHandle, TextureOptions};
use image::{DynamicImage, ImageDecoder, ImageReader, imageops::FilterType};

/// Every GPU that runs OpenGL 3 handles this. Used until the real limit is
/// known, so decoding can start before the window exists.
const ASSUMED_MAX_SIDE: usize = 16384;

pub struct Frame {
    pub texture: TextureHandle,
}

#[derive(Clone)]
pub struct Loaded {
    pub frames: Arc<[Frame]>,
    /// Size of the file on disk after EXIF rotation, even if the texture had to be shrunk.
    pub size: [usize; 2],
    pub bytes: u64,
}

/// What a worker hands back. Textures are created on the UI thread because
/// workers start before the window and its GPU context exist.
pub struct Decoded {
    frames: Vec<(ColorImage, Duration)>,
    size: [usize; 2],
    bytes: u64,
}

impl Decoded {
    fn largest_side(&self) -> usize {
        self.frames
            .iter()
            .map(|(img, _)| img.size[0].max(img.size[1]))
            .max()
            .unwrap_or(0)
    }

    /// Uploads the frames. Returns `None` if they are larger than the GPU allows,
    /// which only happens for photos decoded before the real limit was known.
    pub fn upload(self, ctx: &egui::Context, name: &Path, max_side: usize) -> Option<Loaded> {
        if self.largest_side() > max_side {
            return None;
        }
        let name = name.to_string_lossy();
        let frames: Vec<Frame> = self
            .frames
            .into_iter()
            .enumerate()
            .map(|(i, (image, _))| Frame {
                texture: ctx.load_texture(format!("{name}#{i}"), image, texture_options()),
            })
            .collect();
        Some(Loaded {
            frames: frames.into(),
            size: self.size,
            bytes: self.bytes,
        })
    }
}

pub type Outcome = Result<Decoded, String>;

#[derive(Default)]
struct Queue {
    jobs: VecDeque<PathBuf>,
    busy: HashSet<PathBuf>,
}

struct Shared {
    queue: Mutex<Queue>,
    wake: Condvar,
    max_side: AtomicUsize,
    ctx: OnceLock<egui::Context>,
}

pub struct Loader {
    shared: Arc<Shared>,
    results: Receiver<(PathBuf, Outcome)>,
}

impl Loader {
    pub fn new() -> Self {
        let shared = Arc::new(Shared {
            queue: Mutex::new(Queue::default()),
            wake: Condvar::new(),
            max_side: AtomicUsize::new(ASSUMED_MAX_SIDE),
            ctx: OnceLock::new(),
        });
        let (tx, results) = channel();
        let workers = std::thread::available_parallelism()
            .map_or(2, |n| n.get().saturating_sub(1))
            .clamp(1, 4);
        for i in 0..workers {
            let shared = shared.clone();
            let tx = tx.clone();
            std::thread::Builder::new()
                .name(format!("decode-{i}"))
                .spawn(move || worker(&shared, &tx))
                .expect("failed to start decode thread");
        }
        Self { shared, results }
    }

    /// Called once the window is up: finished photos now wake the UI,
    /// and later decodes respect the real texture limit.
    pub fn attach(&self, ctx: &egui::Context, max_side: usize) {
        self.shared.max_side.store(max_side, Ordering::Relaxed);
        if self.shared.ctx.set(ctx.clone()).is_ok() {
            ctx.request_repaint();
        }
    }

    pub fn max_side(&self) -> usize {
        self.shared.max_side.load(Ordering::Relaxed)
    }

    /// Replaces the pending queue. Paths are decoded in the given order,
    /// files that are already being decoded are not queued twice.
    pub fn want(&self, paths: impl IntoIterator<Item = PathBuf>) {
        let mut q = self.shared.queue.lock().unwrap();
        q.jobs.clear();
        for p in paths {
            if !q.busy.contains(&p) && !q.jobs.contains(&p) {
                q.jobs.push_back(p);
            }
        }
        drop(q);
        self.shared.wake.notify_all();
    }

    pub fn poll(&self) -> impl Iterator<Item = (PathBuf, Outcome)> + '_ {
        self.results.try_iter()
    }
}

fn worker(shared: &Shared, tx: &Sender<(PathBuf, Outcome)>) {
    loop {
        let path = {
            let mut q = shared.queue.lock().unwrap();
            loop {
                if let Some(p) = q.jobs.pop_front() {
                    q.busy.insert(p.clone());
                    break p;
                }
                q = shared.wake.wait(q).unwrap();
            }
        };

        let outcome = decode(&path, shared.max_side.load(Ordering::Relaxed));

        shared.queue.lock().unwrap().busy.remove(&path);
        if tx.send((path, outcome)).is_err() {
            return;
        }
        if let Some(ctx) = shared.ctx.get() {
            ctx.request_repaint();
        }
    }
}

fn texture_options() -> TextureOptions {
    TextureOptions {
        mipmap_mode: Some(TextureFilter::Linear),
        ..TextureOptions::LINEAR
    }
}

fn decode(path: &Path, max_side: usize) -> Result<Decoded, String> {
    let bytes = std::fs::metadata(path).map_err(|e| e.to_string())?.len();
    let reader = ImageReader::open(path)
        .and_then(|r| r.with_guessed_format())
        .map_err(|e| e.to_string())?;

    let mut decoder = reader.into_decoder().map_err(|e| e.to_string())?;
    // A broken EXIF block should not stop the photo from showing.
    let orientation = decoder.orientation().ok();
    let mut img = DynamicImage::from_decoder(decoder).map_err(|e| e.to_string())?;
    if let Some(o) = orientation {
        img.apply_orientation(o);
    }

    let size = [img.width() as usize, img.height() as usize];
    let img = fit_texture(img, max_side);
    let dims = [img.width() as usize, img.height() as usize];
    let image = match img {
        DynamicImage::ImageRgb8(rgb) => ColorImage::from_rgb(dims, rgb.as_raw()),
        other => ColorImage::from_rgba_unmultiplied(dims, other.into_rgba8().as_raw()),
    };
    Ok(Decoded {
        frames: vec![(image, Duration::ZERO)],
        size,
        bytes,
    })
}

fn fit_texture(img: DynamicImage, max_side: usize) -> DynamicImage {
    if img.width().max(img.height()) as usize > max_side {
        img.resize(max_side as u32, max_side as u32, FilterType::Triangle)
    } else {
        img
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("guren-{tag}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn wait_for(loader: &Loader, n: usize) -> Vec<(PathBuf, Outcome)> {
        let mut got = Vec::new();
        let deadline = Instant::now() + Duration::from_secs(10);
        while got.len() < n && Instant::now() < deadline {
            got.extend(loader.poll());
            std::thread::sleep(Duration::from_millis(5));
        }
        got
    }

    #[test]
    fn decodes_before_a_window_exists_and_reports_failures() {
        let dir = temp_dir("still");
        let good = dir.join("a.png");
        image::RgbImage::new(30, 20).save(&good).unwrap();
        let bad = dir.join("b.jpg");
        std::fs::write(&bad, b"not a jpeg").unwrap();

        let loader = Loader::new();
        loader.want([good.clone(), bad.clone()]);
        let mut got = wait_for(&loader, 2);
        std::fs::remove_dir_all(&dir).ok();

        let bad_at = got.iter().position(|(p, _)| *p == bad).unwrap();
        assert!(got.remove(bad_at).1.is_err());
        let (path, outcome) = got.pop().unwrap();
        let ctx = egui::Context::default();
        let loaded = outcome.unwrap().upload(&ctx, &path, 64).unwrap();
        assert_eq!(loaded.size, [30, 20]);
        assert_eq!(loaded.frames[0].texture.size(), [30, 20]);
    }

    #[test]
    fn photo_larger_than_the_gpu_limit_is_not_uploaded() {
        let decoded = Decoded {
            frames: vec![(
                ColorImage::filled([40, 10], egui::Color32::BLACK),
                Duration::ZERO,
            )],
            size: [40, 10],
            bytes: 0,
        };
        let ctx = egui::Context::default();
        assert!(decoded.upload(&ctx, Path::new("x"), 32).is_none());
    }
}
