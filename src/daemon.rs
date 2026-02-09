use crate::cli::Cli;
use crate::traits::Backend;
use crate::wallpaper::WallpaperCache;
use std::time::Duration;
use tokio::process::{Child, Command};
use tokio::time::sleep;

pub async fn run_loop<B: Backend>(cli: Cli, backend: B) -> anyhow::Result<()> {
    // 1. Initialize Cache (Scan disk once)
    let cache = WallpaperCache::new(&cli.wallpaper_dir)?;

    // 2. Parse time
    let period: Duration =
        parse_duration::parse(&cli.time).map_err(|e| anyhow::anyhow!("invalid duration: {e}"))?;

    // 3. Keep track of the running background process
    let mut current_swaybg: Option<Child> = None;

    log::info!("Starting wallpaper daemon. Interval: {period:?}");

    loop {
        // A. Get monitors using the abstract backend
        let monitors = match backend.get_active_monitors().await {
            Ok(m) => m,
            Err(e) => {
                log::error!("Failed to get monitors: {e}. Retrying in 5s...");
                sleep(Duration::from_secs(5)).await;
                continue;
            }
        };

        // B. Build arguments
        let mut args = Vec::new();
        for monitor in monitors {
            let img = cache.pick_random();
            // Important: swaybg needs absolute paths usually
            if let Ok(abs_path) = img.canonicalize() {
                args.push("-o".to_string());
                args.push(monitor);
                args.push("-m".to_string());
                args.push("fill".to_string());
                args.push("-i".to_string());
                args.push(abs_path.to_string_lossy().to_string());
            }
        }

        // C. Manage the process
        if !args.is_empty() {
            // Kill previous instance
            if let Some(mut child) = current_swaybg.take() {
                let _ = child.kill().await;
                let _ = child.wait().await; // Reap zombie process
            }

            // Spawn new instance
            // Note: We do NOT use .status() or .output() because those block!
            // We use .spawn() to let it run in background.
            let child_result = Command::new("swaybg")
                .args(&args)
                .kill_on_drop(true) // If randpaper crashes, kill swaybg
                .spawn();

            match child_result {
                Ok(child) => current_swaybg = Some(child),
                Err(e) => log::error!("Failed to spawn swaybg: {e}"),
            }
        }

        // D. Sleep
        sleep(period).await;
    }
}
