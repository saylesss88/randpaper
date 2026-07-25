use std::time::Duration;

use tokio::{
    signal::unix::{SignalKind, signal},
    time::sleep,
};

use super::render;
use crate::{
    cli::Config, daemon_lock::session_key, theme::update_theme_file, traits::Backend,
    wallpaper::WallpaperCache,
};
use randpaper_ipc::{self, DaemonCommand, DaemonState};

/// Runs the persistent background process that cycles wallpapers and themes.
///
/// # Errors
///
/// Returns an error if theme/cache initialization fails, the configuration contains an
/// invalid duration format, renderer initialization fails, or if setting up the `SIGUSR1`
/// signal handler encounters an OS error.
pub async fn run_loop<B: Backend>(config: Config, backend: B) -> anyhow::Result<()> {
    crate::theme::ensure_theme_exists()?;
    let cache = WallpaperCache::new(&config.wallpaper_dir)?;
    let mut renderer = render::Renderer::new(&config).await?;
    let period: Duration = humantime::parse_duration(
        config
            .time
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("daemon mode requires --time"))?,
    )
    .map_err(|e| anyhow::anyhow!("invalid duration: {e}"))?;

    let (cmd_tx, mut cmd_rx) = tokio::sync::mpsc::channel::<DaemonCommand>(8);
    let key = session_key();
    tokio::spawn(async move {
        if let Err(e) = randpaper_ipc::listen_for_ipc(cmd_tx, &key).await {
            log::error!("IPC listener exited: {e:#}");
        }
    });
    let mut sig_usr1 = signal(SignalKind::user_defined1())?;
    let mut daemon_state = DaemonState { paused: false };

    // Apply wallpaper immediately on startup
    let monitors = backend.get_active_monitors().await?;
    let img = cache.pick_random();
    let _ = update_theme_file(img);

    if let Err(e) = renderer.apply(&config, &cache, &monitors).await {
        log::error!("Failed to apply wallpaper: {e:#}. Retrying next cycle.");
        return Err(e);
    }

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

        #[allow(clippy::needless_continue)]
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
            if let Err(e) = renderer.apply(&config, &cache, &monitors).await {
                log::error!("Failed to apply wallpaper: {e:#}. Retrying next cycle");
                continue;
            }
        }
    }
}
