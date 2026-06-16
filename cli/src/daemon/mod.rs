use crate::cli::Config;
use crate::daemon::ipc::{DaemonCommand, DaemonState};
use crate::theme::update_theme_file;
use crate::traits::Backend;
use crate::wallpaper::WallpaperCache;

use std::time::Duration;
use tokio::signal::unix::{SignalKind, signal};
use tokio::time::sleep;

pub mod ipc;
mod render;

// Re-exports
pub use render::awww::detect_awww_binary;
pub use render::awww::ensure_awww_daemon;

/// Runs the persistent background process that cycles wallpapers and themes.
///
/// The daemon performs the following:
/// 1. Initializes the wallpaper cache and determines the rotation frequency.
/// 2. Sets up a listener for `SIGUSR1` to allow manual skips.
/// 3. Enters an infinite loop that updates themes and wallpapers based on the timer.
pub async fn run_loop<B: Backend>(config: Config, backend: B) -> anyhow::Result<()> {
    crate::theme::ensure_theme_exists()?;
    let cache = WallpaperCache::new(&config.wallpaper_dir)?;
    let period: Duration =
        humantime::parse_duration(config.time.as_ref().expect("daemon mode requires --time"))
            .map_err(|e| anyhow::anyhow!("invalid duration: {e}"))?;
    let mut renderer = render::Renderer::new(&config).await?;
    let (cmd_tx, mut cmd_rx) = tokio::sync::mpsc::channel::<DaemonCommand>(8);
    tokio::spawn(async move {
        if let Err(e) = ipc::listen_for_ipc(cmd_tx).await {
            log::error!("IPC listener exited: {e:#}");
        }
    });
    let mut sig_usr1 = signal(SignalKind::user_defined1())?;
    let mut daemon_state = DaemonState { paused: false };

    // Apply wallpaper immediately on startup
    let monitors = backend.get_active_monitors().await?;
    let img = cache.pick_random();
    let _ = update_theme_file(img);
    renderer.apply(&config, &cache, &monitors).await?;

    loop {
        log::debug!(
            "Waiting for timer/signal/IPC, paused={}",
            daemon_state.paused
        );

        let mut should_cycle = false;

        tokio::select! {
            () = sleep(period) => {
                log::debug!("Timer fired");
                should_cycle = true;
            }
            _ = sig_usr1.recv() => {
                log::info!("Received SIGUSR1. Cycling wallpaper immediately.");
                should_cycle = true;
            }
            Some(cmd) = cmd_rx.recv() => {
                log::debug!("Recieved IPC command: {cmd:?}");
                match cmd {
                    DaemonCommand::Next => {
                        log::info!("IPC next wallpaper");
                        should_cycle = true;
                    }
                    DaemonCommand::Pause => {
                        log::info!("IPC pausing");
                        daemon_state.paused = true;
                    }
                    DaemonCommand::Resume => {
                        log::info!("IPC resuming");
                        daemon_state.paused = false;
                        should_cycle = false;
                    }
                    DaemonCommand::Status(reply) => {
                        let msg = format!("running, paused={}", daemon_state.paused);
                        let _ = reply.send(msg);
                    }
                }
            }
        }

        if should_cycle && !daemon_state.paused {
            log::debug!("Cycling wallpaper");
            let monitors = match backend.get_active_monitors().await {
                Ok(m) => m,
                Err(e) => {
                    log::error!("Failed to get monitors: {e}. Retrying next cycle.");
                    continue;
                }
            };
            let img = cache.pick_random();
            let _ = update_theme_file(img);
            renderer.apply(&config, &cache, &monitors).await?;
        }
    }
}
