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
use clap::Parser;
use cli::{BackendType, Cli, RendererType};
use tokio::process::Command;

async fn oneshot_mode(cli: &Cli) -> anyhow::Result<()> {
    log::info!("One-shot mode: picking wallpaper once and exiting");

    let cache = WallpaperCache::new(&cli.wallpaper_dir)?;

    // Get monitors using the appropriate backend
    let monitors = match cli.backend {
        BackendType::Hyprland => HyprlandBackend.get_active_monitors().await?,
        BackendType::Sway => {
            let backend = SwayBackend {
                outputs_override: cli.outputs.clone(),
            };
            backend.get_active_monitors().await?
        }
    };

    // Pick wallpaper and update theme
    let img = cache.pick_random();
    theme::update_theme_file(img)?;

    // Set wallpaper
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

            // Kill existing swaybg instances
            let _ = Command::new("pkill").args(["-x", "swaybg"]).status().await;

            // Start new swaybg
            Command::new("swaybg").args(&args).spawn()?;
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
                    .await?;
            }
        }
    }

    log::info!("Wallpaper and theme updated. Exiting.");
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    let cli = Cli::parse();

    crate::theme::ensure_theme_exists()?;

    // One-shot mode: no --time flag
    if cli.time.is_none() {
        return oneshot_mode(&cli).await;
    }

    // Daemon mode: --time flag present
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
