#![allow(clippy::multiple_crate_versions)]
mod backends;
mod cli;
mod daemon;
mod daemon_lock;
mod theme;
mod traits;
mod wallpaper;

use crate::backends::hyprland::HyprlandBackend;
use crate::backends::sway::SwayBackend;
use crate::traits::Backend;
use crate::wallpaper::WallpaperCache;
use anyhow::Context;
use cli::{BackendType, Config, RendererType};
use tokio::process::Command;

/// Executes a single wallpaper and theme update.
///
/// This mode is triggered when the user does not provide a `--time` interval.
/// It detects monitors via the chosen backend, picks a random wallpaper,
/// updates the system themes, and invokes the selected renderer.
async fn oneshot_mode(config: &Config) -> anyhow::Result<()> {
    log::info!("One-shot mode: picking wallpaper once and exiting");

    // Initialize the wallpaper cache from the provided directory
    let cache = WallpaperCache::new(&config.wallpaper_dir)?;

    // 1. Identify active monitors based on the user-selected backend (Hyprland or Sway)
    let monitors = match config.backend {
        BackendType::Hyprland => HyprlandBackend.get_active_monitors().await?,
        BackendType::Sway => {
            let backend = SwayBackend {
                outputs_override: config.outputs.clone(),
            };
            backend.get_active_monitors().await?
        }
    };

    // 2. Pick wallpaper and generate the theme files (Waybar, Terminals)
    let img = cache.pick_random();
    theme::update_theme_file(img)?;

    // 3. Apply the wallpaper using the selected renderer (swaybg or awww)
    match config.renderer {
        RendererType::Swaybg => {
            let mut args = Vec::new();
            for monitor in &monitors {
                let img = cache.pick_random();
                let abs_path = img.canonicalize()?;
                args.extend_from_slice(&[
                    "-o".to_string(),
                    monitor.clone(),
                    "-m".to_string(),
                    "fill".to_string(),
                    "-i".to_string(),
                    abs_path.to_string_lossy().to_string(),
                ]);
            }

            // Cleanup old instances to prevent resource leaks/stacking
            let _ = Command::new("pkill")
                .args(["-x", "swaybg"])
                .status()
                .await
                .context("oneshot: pkill -x swaybg")?;

            // Start new swaybg
            // Command::new("swaybg").args(&args).spawn()?;
            Command::new("swaybg")
                .args(&args)
                .spawn()
                .context("oneshot: spawn swaybg")?;
        }

        RendererType::Awww => {
            let awww_bin = daemon::detect_awww_binary().await; // Use from daemon module
            daemon::ensure_awww_daemon(&awww_bin).await?;
            let step = config.transition_step.to_string();
            let fps = config.transition_fps.to_string();

            for monitor in &monitors {
                let img = cache.pick_random();
                Command::new(&awww_bin)
                    .arg("img")
                    .arg(img)
                    .arg("-o")
                    .arg(monitor)
                    .arg("--transition-type")
                    .arg(&config.transition_type)
                    .arg("--transition-step")
                    .arg(&step)
                    .arg("--transition-fps")
                    .arg(&fps)
                    .status()
                    .await
                    .with_context(|| format!("oneshot: awww img -o {monitor}"))?;
            }
        }
    }

    log::info!("Wallpaper and theme updated. Exiting.");
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();

    let config = Config::new()?;
    crate::theme::ensure_theme_exists()?;

    if !config.daemon {
        return oneshot_mode(&config).await;
    }

    // --daemon mode
    if config.time.is_none() {
        anyhow::bail!("--daemon requires time to be set (config.toml or --time)");
    }

    // One daemon per login session: use a runtime-dir lock (per-session).
    // XDG_RUNTIME_DIR is the right place for per-session runtime state.
    let Some(_guard) = crate::daemon_lock::single_instance_guard()? else {
        return Ok(());
    };

    match config.backend {
        BackendType::Hyprland => {
            log::info!("Using Hyprland backend");
            daemon::run_loop(config, HyprlandBackend).await?;
        }
        BackendType::Sway => {
            log::info!("Using Sway backend");
            let backend = SwayBackend {
                outputs_override: config.outputs.clone(),
            };
            daemon::run_loop(config, backend).await?;
        }
    }

    Ok(())
}
