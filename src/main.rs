#![allow(clippy::multiple_crate_versions)]
mod backends;
mod cli;
mod daemon;
mod theme;
mod traits;
mod wallpaper;

use crate::backends::hyprland::HyprlandBackend;
use crate::backends::sway::SwayBackend;
use crate::traits::Backend;
use crate::wallpaper::WallpaperCache;
use anyhow::Context;
use clap::Parser;
use cli::{BackendType, Cli, RendererType};
use tokio::process::Command;

/// Executes a single wallpaper and theme update.
///
/// This mode is triggered when the user does not provide a `--time` interval.
/// It detects monitors via the chosen backend, picks a random wallpaper,
/// updates the system themes, and invokes the selected renderer.
async fn oneshot_mode(cli: &Cli) -> anyhow::Result<()> {
    log::info!("One-shot mode: picking wallpaper once and exiting");

    // Initialize the wallpaper cache from the provided directory
    let cache = WallpaperCache::new(&cli.wallpaper_dir)?;

    // 1. Identify active monitors based on the user-selected backend (Hyprland or Sway)
    let monitors = match cli.backend {
        BackendType::Hyprland => HyprlandBackend.get_active_monitors().await?,
        BackendType::Sway => {
            let backend = SwayBackend {
                outputs_override: cli.outputs.clone(),
            };
            backend.get_active_monitors().await?
        }
    };

    // 2. Pick wallpaper and generate the theme files (Waybar, Terminals)
    let img = cache.pick_random();
    theme::update_theme_file(img)?;

    // 3. Apply the wallpaper using the selected renderer (swaybg or swww)
    match cli.renderer {
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

        RendererType::Swww => {
            let swww_bin = daemon::detect_swww_binary().await; // Use from daemon module
            let step = cli.transition_step.to_string();
            let fps = cli.transition_fps.to_string();

            for monitor in &monitors {
                let img = cache.pick_random();
                Command::new(&swww_bin)
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
                    .status()
                    .await
                    .with_context(|| format!("oneshot: swww img -o {monitor}"))?;
            }
        }
    }

    log::info!("Wallpaper and theme updated. Exiting.");
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging (controlled via RUST_LOG env var)
    env_logger::init();

    // Parse command line arguments with clap
    let cli = Cli::parse();

    // Ensure initial theme file exists to prevent Waybar `@import` crashes
    crate::theme::ensure_theme_exists()?;

    // Determine execution mode:
    // If no time interval is provided, run once and exit.
    // Otherwise, hand over control to the daemon's infinite loop.
    if cli.time.is_none() {
        return oneshot_mode(&cli).await;
    }

    // Enter Daemon mode: --time flag present
    match cli.backend {
        BackendType::Hyprland => {
            log::info!("Using Hyprland backend");
            daemon::run_loop(cli, HyprlandBackend).await?;
        }
        BackendType::Sway => {
            log::info!("Using Sway backend");
            let backend = SwayBackend {
                outputs_override: cli.outputs.clone(),
            };
            daemon::run_loop(cli, backend).await?;
        }
    }

    Ok(())
}
