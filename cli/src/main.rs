#![allow(clippy::multiple_crate_versions)]
mod backends;
mod cli;
mod daemon;
mod daemon_lock;
#[cfg(any(feature = "waybar", feature = "terminals"))]
mod theme;
mod traits;
mod wallpaper;

#[cfg(feature = "native-renderer")]
use std::sync::{Arc, atomic::AtomicBool};

use anyhow::Context;
use clap::Parser;
use tokio::net::UnixStream;
use tokio::process::Command as TokioCommand;

use cli::{BackendType, Cli, Command, Config, RendererType};
use randpaper_ipc::find_socket;
#[cfg(feature = "native-renderer")]
use randpaper_lib::layer;

use crate::backends::{hyprland::HyprlandBackend, mango::MangoBackend, sway::SwayBackend};
use crate::daemon_lock::session_key;
use crate::traits::Backend;
use crate::wallpaper::WallpaperCache;

/// # Errors
///
/// Returns an error if it can't find the daemon socket
pub async fn send_ipc_command(cmd: &str) -> anyhow::Result<()> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let socket_path = find_socket(&session_key())?;
    let mut stream = UnixStream::connect(&socket_path)
        .await
        .with_context(|| format!("Failed to connect to socket at {}", socket_path.display()))?;

    stream.write_all(cmd.as_bytes()).await?;
    stream.shutdown().await?;

    if cmd == "status" {
        let mut reply = String::new();
        stream.read_to_string(&mut reply).await?;
        if !reply.trim().is_empty() {
            println!("{reply}");
        }
    }

    Ok(())
}
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
        BackendType::Mango => MangoBackend.get_active_monitors().await?,
        BackendType::Sway => {
            let backend = SwayBackend {
                outputs_override: config.outputs.clone(),
            };
            backend.get_active_monitors().await?
        }
    };

    // 2. Pick wallpaper and generate the theme files (Waybar, Terminals)
    #[cfg(any(feature = "waybar", feature = "terminals"))]
    let img = cache.pick_random();
    #[cfg(any(feature = "waybar", feature = "terminals"))]
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
            let _ = TokioCommand::new("pkill")
                .args(["-x", "swaybg"])
                .status()
                .await
                .context("oneshot: pkill -x swaybg")?;

            // Start new swaybg
            // Command::new("swaybg").args(&args).spawn()?;
            TokioCommand::new("swaybg")
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
                TokioCommand::new(&awww_bin)
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

        #[cfg(feature = "native-renderer")]
        RendererType::Native => {
            // Spawn one thread per monitor so they run concurrently.
            let mut handles = Vec::new();
            for monitor in monitors {
                let img_path = cache.pick_random().to_path_buf();
                handles.push(std::thread::spawn(move || {
                    layer::render_wallpaper(
                        &img_path,
                        Some(&monitor),
                        &Arc::new(AtomicBool::new(false)),
                    )
                    .expect("native renderer failed");
                }));
            }
            for h in handles {
                h.join().ok();
            }
        }
    }

    log::info!("Wallpaper and theme updated. Exiting.");
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli_args = Cli::parse();

    env_logger::init();

    if let Some(cmd) = &cli_args.command {
        return match cmd {
            Command::Next => send_ipc_command("next").await,
            Command::Pause => send_ipc_command("pause").await,
            Command::Resume => send_ipc_command("resume").await,
            Command::Status => send_ipc_command("status").await,
        };
    }

    let config = Config::from_cli(cli_args)?;
    #[cfg(feature = "waybar")]
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
        log::info!("Another randpaper daemon is already running. Exiting.");
        return Ok(());
    };

    match config.backend {
        BackendType::Hyprland => {
            log::info!("Using Hyprland backend");
            daemon::run_loop(config, HyprlandBackend).await?;
        }
        BackendType::Mango => {
            log::info!("Using MangoWM backend");
            daemon::run_loop(config, MangoBackend).await?;
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
