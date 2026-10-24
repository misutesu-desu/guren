use std::collections::{HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Condvar, Mutex};

use eframe::egui::{self, ColorImage, TextureFilter, TextureHandle, TextureOptions};
use image::{DynamicImage, ImageDecoder, ImageReader, imageops::FilterType};

#[derive(Clone)]
pub struct Loaded {
    pub texture: TextureHandle,
    /// Size of the file on disk after EXIF rotation, even if the texture had to be shrunk.
    pub size: [usize; 2],
    pub bytes: u64,
}

pub type Outcome = Result<Loaded, String>;

#[derive(Default)]
struct Queue {
    jobs: VecDeque<PathBuf>,
    busy: HashSet<PathBuf>,
}

struct Shared {
    queue: Mutex<Queue>,
    wake: Condvar,
}

pub struct Loader {
    shared: Arc<Shared>,
    results: Receiver<(PathBuf, Outcome)>,
}

impl Loader {
    pub fn new(ctx: &egui::Context) -> Self {
        let shared = Arc::new(Shared {
            queue: Mutex::new(Queue::default()),
            wake: Condvar::new(),
        });
        let (tx, results) = channel();
        let max_side = ctx.input(|i| i.max_texture_side);
        let workers = std::thread::available_parallelism()
            .map_or(2, |n| n.get().saturating_sub(1))
            .clamp(1, 4);
        for i in 0..workers {
            let shared = shared.clone();
            let tx = tx.clone();
            let ctx = ctx.clone();
            std::thread::Builder::new()
                .name(format!("decode-{i}"))
                .spawn(move || worker(&shared, &tx, &ctx, max_side))
                .expect("failed to start decode thread");
        }
        Self { shared, results }
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

fn worker(shared: &Shared, tx: &Sender<(PathBuf, Outcome)>, ctx: &egui::Context, max_side: usize) {
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

        let outcome = decode(&path, max_side).map(|(image, size, bytes)| Loaded {
            texture: ctx.load_texture(path.to_string_lossy(), image, texture_options()),
            size,
            bytes,
        });

        shared.queue.lock().unwrap().busy.remove(&path);
        if tx.send((path, outcome)).is_err() {
            return;
        }
        ctx.request_repaint();
    }
}

fn texture_options() -> TextureOptions {
    TextureOptions {
        mipmap_mode: Some(TextureFilter::Linear),
        ..TextureOptions::LINEAR
    }
}

fn decode(path: &Path, max_side: usize) -> Result<(ColorImage, [usize; 2], u64), String> {
    let bytes = std::fs::metadata(path).map_err(|e| e.to_string())?.len();
    let mut decoder = ImageReader::open(path)
        .and_then(|r| r.with_guessed_format())
        .map_err(|e| e.to_string())?
        .into_decoder()
        .map_err(|e| e.to_string())?;
    // A broken EXIF block should not stop the photo from showing.
    let orientation = decoder.orientation().ok();
    let mut img = DynamicImage::from_decoder(decoder).map_err(|e| e.to_string())?;
    if let Some(o) = orientation {
        img.apply_orientation(o);
    }

    let size = [img.width() as usize, img.height() as usize];
    if size[0].max(size[1]) > max_side {
        img = img.resize(max_side as u32, max_side as u32, FilterType::Triangle);
    }

    let dims = [img.width() as usize, img.height() as usize];
    let image = match img {
        DynamicImage::ImageRgb8(rgb) => ColorImage::from_rgb(dims, rgb.as_raw()),
        other => ColorImage::from_rgba_unmultiplied(dims, other.into_rgba8().as_raw()),
    };
    Ok((image, size, bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    #[test]
    fn decodes_in_background_and_reports_failures() {
        let dir = std::env::temp_dir().join(format!("guren-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let good = dir.join("a.png");
        image::RgbImage::new(30, 20).save(&good).unwrap();
        let bad = dir.join("b.jpg");
        std::fs::write(&bad, b"not a jpeg").unwrap();

        let ctx = egui::Context::default();
        let loader = Loader::new(&ctx);
        loader.want([good.clone(), bad.clone()]);

        let mut got = Vec::new();
        let deadline = Instant::now() + Duration::from_secs(10);
        while got.len() < 2 && Instant::now() < deadline {
            got.extend(loader.poll());
            std::thread::sleep(Duration::from_millis(5));
        }
        std::fs::remove_dir_all(&dir).ok();

        let find = |p: &Path| got.iter().find(|(q, _)| q == p).map(|(_, o)| o);
        let loaded = find(&good).unwrap().as_ref().unwrap();
        assert_eq!(loaded.size, [30, 20]);
        assert_eq!(loaded.texture.size(), [30, 20]);
        assert!(find(&bad).unwrap().is_err());
    }
}
