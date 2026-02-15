use crate::wallpaper::WallpaperCache;
use std::path::PathBuf;
use tokio::process::{Child, Command};

/// Constructs the command-line arguments for `swaybg`.
///
/// It maps each monitor to a randomly selected wallpaper from the cache.
///
/// # Arguments
/// * `monitors` - Slice of monitor names to apply wallpapers to.
/// * `pick_random` - Closure that returns a path to a random image.
/// * `mode` - Closure that returns the scaling mode (e.g., "fill").
fn build_swaybg_args<F, M>(monitors: &[String], pick_random: F, mode: M) -> Vec<String>
where
    F: Fn() -> PathBuf,
    M: Fn() -> String,
{
    let mut args = Vec::new();
    for monitor in monitors {
        let img = pick_random();
        let Ok(abs_path) = img.canonicalize() else {
            continue;
        };

        args.push("-o".to_string());
        args.push(monitor.clone());
        args.push("-m".to_string());
        args.push(mode());
        args.push("-i".to_string());
        args.push(abs_path.to_string_lossy().to_string());
    }
    args
}

/// Applies wallpapers using `swaybg`.
///
/// Since `swaybg` does not have a daemon, this function:
/// 1. Kills the previously running `swaybg` process (if any).
/// 2. Spawns a new `swaybg` process as a long-running child.
///
/// # Errors
/// Returns an error if the command fails to spawn.
pub async fn apply(
    cache: &WallpaperCache,
    monitors: &[String],
    current: &mut Option<Child>,
) -> anyhow::Result<()> {
    let pick_random = || cache.pick_random().to_path_buf();
    let args = build_swaybg_args(monitors, pick_random, || "fill".to_string());

    if args.is_empty() {
        return Ok(());
    }

    // Terminate the existing swaybg process before starting a new one
    // to prevent multiple instances from overlapping or wasting resources.
    if let Some(mut child) = current.take() {
        let _ = child.kill().await;
        let _ = child.wait().await;
    }

    if let Ok(child) = Command::new("swaybg")
        .args(&args)
        .kill_on_drop(true)
        .spawn()
    {
        *current = Some(child);
    }

    Ok(())
}
