use crate::wallpaper::WallpaperCache;
use std::sync::{Arc, atomic::AtomicBool};

/// Applies wallpapers using the built-in native Wayland renderer.
///
/// Each monitor gets its own `spawn_blocking` thread running the Wayland
/// event loop for that surface. When called again (rotation), the old
/// handles are aborted before new ones are spawned.
///
/// # Errors
/// Returns an error if no valid image paths can be resolved.
pub fn apply(
    cache: &WallpaperCache,
    monitors: &[String],
    current_handles: &mut Vec<tokio::task::JoinHandle<()>>,
) {
    for handle in current_handles.drain(..) {
        handle.abort();
    }
    for monitor in monitors {
        let img_path = cache
            .pick_random()
            .canonicalize()
            .unwrap_or_else(|_| cache.pick_random().to_path_buf());
        let monitor_name = monitor.clone();
        let handle = tokio::task::spawn_blocking(move || {
            if let Err(e) = crate::layer::render_wallpaper(
                &img_path,
                Some(&monitor_name),
                &Arc::new(AtomicBool::new(false)),
            ) {
                log::error!("native renderer error on {monitor_name}: {e:#}");
            }
        });
        current_handles.push(handle);
    }
}
