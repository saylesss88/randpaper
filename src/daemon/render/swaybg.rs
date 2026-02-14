use crate::wallpaper::WallpaperCache;
use std::path::PathBuf;
use tokio::process::{Child, Command};

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
