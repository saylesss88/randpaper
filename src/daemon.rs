use crate::cli::{Cli, RendererType};
use crate::theme::update_theme_file;
use crate::traits::Backend;
use crate::wallpaper::WallpaperCache;
use std::time::Duration;
use tokio::process::{Child, Command};
use tokio::signal::unix::{SignalKind, signal};
use tokio::time::sleep;

async fn detect_swww_binary() -> String {
    // Try 'swww' first
    if Command::new("swww").arg("--help").output().await.is_ok() {
        return "swww".to_string();
    }
    // Try 'awww' second
    if Command::new("awww").arg("--help").output().await.is_ok() {
        return "awww".to_string();
    }
    // Default fallback (log a warning if neither found, but return swww)
    log::warn!("Neither 'swww' nor 'awww' found. Defaulting to 'swww'.");
    "swww".to_string()
}

pub async fn run_loop<B: Backend>(cli: Cli, backend: B) -> anyhow::Result<()> {
    let cache = WallpaperCache::new(&cli.wallpaper_dir)?;
    let period: Duration =
        parse_duration::parse(&cli.time).map_err(|e| anyhow::anyhow!("invalid duration: {e}"))?;

    let mut current_swaybg: Option<Child> = None;

    // DETECT BINARY NAME ONCE
    let swww_bin = if cli.renderer == RendererType::Swww {
        detect_swww_binary().await
    } else {
        String::new() // Not needed for swaybg
    };

    log::info!(
        "Starting daemon. Interval: {:?}, Renderer: {:?}",
        period,
        cli.renderer
    );

    // If using SWWW, we should ensure the daemon is initialized once
    if cli.renderer == RendererType::Swww {
        let daemon_cmd = format!("{swww_bin}-daemon"); // e.g., "swww-daemon" or "awww-daemon"
        // Try to start the daemon. Use spawn so it runs in background.
        // We ignore the result because it might already be running.
        let _ = Command::new(&daemon_cmd).spawn();

        sleep(Duration::from_millis(500)).await;
    }

    let mut sig_usr1 = signal(SignalKind::user_defined1())?;

    loop {
        // A. Get Monitors
        let monitors = match backend.get_active_monitors().await {
            Ok(m) => m,
            Err(e) => {
                log::error!("Failed to get monitors: {e}. Retrying in 5s...");
                sleep(Duration::from_secs(5)).await;
                continue;
            }
        };

        match cli.renderer {
            // ==========================================
            // LOGIC FOR SWAYBG (The Daemon Approach)
            // ==========================================
            RendererType::Swaybg => {
                let mut args = Vec::new();
                for (i, monitor) in monitors.iter().enumerate() {
                    let img = cache.pick_random();
                    if i == 0
                        && let Err(e) = update_theme_file(img)
                    {
                        log::warn!("{e}");
                    }

                    if let Ok(abs_path) = img.canonicalize() {
                        args.push("-o".to_string());
                        args.push(monitor.clone());
                        args.push("-m".to_string());
                        args.push("fill".to_string());
                        args.push("-i".to_string());
                        args.push(abs_path.to_string_lossy().to_string());
                    }
                }

                if !args.is_empty() {
                    if let Some(mut child) = current_swaybg.take() {
                        let _ = child.kill().await;
                        let _ = child.wait().await;
                    }
                    let child = Command::new("swaybg")
                        .args(&args)
                        .kill_on_drop(true)
                        .spawn();
                    if let Ok(c) = child {
                        current_swaybg = Some(c);
                    }
                }
            }

            // ==========================================
            // LOGIC FOR SWWW (The Client Approach)
            // ==========================================
            RendererType::Swww => {
                for (i, monitor) in monitors.iter().enumerate() {
                    let img = cache.pick_random();
                    if i == 0
                        && let Err(e) = update_theme_file(img)
                    {
                        log::warn!("{e}");
                    }

                    if let Ok(abs_path) = img.canonicalize() {
                        // Use the DETECTED binary name here
                        let _ = Command::new(&swww_bin)
                            .args([
                                "img",
                                &abs_path.to_string_lossy(),
                                "-o",
                                monitor,
                                "--transition-type",
                                "fade",
                                "--transition-step",
                                "90",
                                "--transition-fps",
                                "60",
                            ])
                            .status()
                            .await;
                    }
                }
            }
        }
        // sleep(period).await;
        log::info!("Sleeping for {period:?}");

        tokio::select! {
            () = sleep(period) => {
                // Timer finished naturally, loop continues to change wallpaper
            }
            _ = sig_usr1.recv() => {
                log::info!("Received skip signal (SIGUSR1). Cycling wallpaper immediately.");
                // Loop continues immediately, effectively skipping the sleep
            }
        }
    }
}
