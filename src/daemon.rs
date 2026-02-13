use crate::cli::{Cli, RendererType};
use crate::theme::update_theme_file;
use crate::traits::Backend;
use crate::wallpaper::WallpaperCache;
use anyhow::Context;
use std::path::PathBuf;
use std::time::Duration;
use tokio::process::{Child, Command};
use tokio::signal::unix::{SignalKind, signal};
use tokio::time::sleep;

pub async fn detect_swww_binary() -> String {
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

async fn ensure_swww_daemon(swww_bin: &str) -> anyhow::Result<()> {
    let daemon_name = format!("{swww_bin}-daemon");

    // Check if daemon is already running using pgrep
    let status = Command::new("pgrep")
        .arg("-x") // Exact match
        .arg(&daemon_name)
        .status()
        .await;

    match status {
        Ok(exit_status) if exit_status.success() => {
            // Daemon is already running
            log::info!("{daemon_name} is already running");
            Ok(())
        }
        _ => {
            // Daemon not running, start it
            log::info!("Starting {daemon_name}...");
            Command::new(&daemon_name)
                .spawn()
                .with_context(|| format!("failed to spawn {daemon_name}"))?;

            // Give it time to initialize
            sleep(Duration::from_millis(500)).await;
            Ok(())
        }
    }
}

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

pub async fn run_loop<B: Backend>(cli: Cli, backend: B) -> anyhow::Result<()> {
    crate::theme::ensure_theme_exists()?;

    let cache = WallpaperCache::new(&cli.wallpaper_dir)?;
    let period: Duration =
        parse_duration::parse(cli.time.as_ref().expect("daemon mode requires --time"))
            .map_err(|e| anyhow::anyhow!("invalid duration: {e}"))?;

    let mut current_swaybg: Option<Child> = None;

    let swww_bin = if cli.renderer == RendererType::Swww {
        detect_swww_binary().await
    } else {
        String::new()
    };

    log::info!(
        "Starting daemon. Interval: {:?}, Renderer: {:?}",
        period,
        cli.renderer
    );

    if cli.renderer == RendererType::Swww {
        ensure_swww_daemon(&swww_bin).await?;
    }

    let mut sig_usr1 = signal(SignalKind::user_defined1())?;

    loop {
        let monitors = match backend.get_active_monitors().await {
            Ok(m) => m,
            Err(e) => {
                log::error!("Failed to get monitors: {e}. Retrying in 5s...");
                sleep(Duration::from_secs(5)).await;
                continue;
            }
        };

        let img = cache.pick_random();
        let _ = update_theme_file(img);

        match cli.renderer {
            RendererType::Swaybg => {
                let pick_random = || cache.pick_random().to_path_buf();
                let args = build_swaybg_args(&monitors, pick_random, || "fill".to_string());

                if !args.is_empty() {
                    if let Some(mut child) = current_swaybg.take() {
                        let _ = child.kill().await;
                        let _ = child.wait().await;
                    }
                    if let Ok(child) = Command::new("swaybg")
                        .args(&args)
                        .kill_on_drop(true)
                        .spawn()
                    {
                        current_swaybg = Some(child);
                    }
                }
            }
            RendererType::Swww => {
                let step = cli.transition_step.to_string();
                let fps = cli.transition_fps.to_string();

                for monitor in &monitors {
                    let img = cache.pick_random(); // already absolute/canonical

                    let out = Command::new(&swww_bin)
                        .arg("img")
                        .arg(img)
                        .arg("-o")
                        .arg(monitor)
                        .arg("--transition-type")
                        .arg(&cli.transition_type)
                        .arg("--transition-step")
                        .arg(&step)
                        .arg("--transition-fps")
                        .arg(&fps)
                        .output()
                        .await
                        .with_context(|| format!("failed to run {swww_bin}"))?;

                    if !out.status.success() {
                        anyhow::bail!(
                            "{swww_bin} failed: {}",
                            String::from_utf8_lossy(&out.stderr)
                        );
                    }
                }
            }
        }

        log::info!("Sleeping for {period:?}");
        tokio::select! {
            () = sleep(period) => {}
            _ = sig_usr1.recv() => {
                log::info!("Received skip signal (SIGUSR1). Cycling wallpaper immediately.");
            }
        }
    }
}
