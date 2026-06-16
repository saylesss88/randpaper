use crate::wallpaper::WallpaperCache;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

pub struct NativeRenderer {
    handles: Vec<tokio::task::JoinHandle<()>>,
    stop_flags: Vec<Arc<AtomicBool>>,
}

impl NativeRenderer {
    pub const fn new() -> Self {
        Self {
            handles: Vec::new(),
            stop_flags: Vec::new(),
        }
    }
    pub fn apply(&mut self, cache: &WallpaperCache, monitors: &[String]) {
        for flag in self.stop_flags.drain(..) {
            flag.store(true, Ordering::Relaxed);
        }
        for handle in self.handles.drain(..) {
            handle.abort();
        }
        for monitor in monitors {
            let img_path = cache
                .pick_random()
                .canonicalize()
                .unwrap_or_else(|_| cache.pick_random().to_path_buf());
            let monitor_name = monitor.clone();
            let stop = Arc::new(AtomicBool::new(false));
            self.stop_flags.push(Arc::clone(&stop));

            let handle = tokio::task::spawn_blocking(move || {
                if let Err(e) =
                    crate::layer::render_wallpaper(&img_path, Some(&monitor_name), &stop)
                {
                    log::error!("native renderer error on {monitor_name}: {e:#}");
                }
            });
            self.handles.push(handle);
        }
    }
}
