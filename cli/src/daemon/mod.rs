use crate::cli::Config;
use crate::theme::update_theme_file;
use crate::traits::Backend;
use crate::wallpaper::WallpaperCache;

use std::path::PathBuf;
use std::time::Duration;
use tokio::signal::unix::{SignalKind, signal};
use tokio::time::sleep;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixListener;
use tokio::sync::mpsc;

#[derive(Debug)]
pub enum DaemonCommand {
    Next,
    Pause,
    Resume,
    Status(tokio::sync::oneshot::Sender<String>),
}

pub fn find_socket() -> Result<PathBuf, BaseDirectoriesError> {
    let xdg_dirs = BaseDirectories::with_prefix("randpaper");
    Ok(xdg_dirs.get_runtime_directory()?.join("randpaper.sock"))
}

async fn listen_for_ipc(tx: mpsc::Sender<DaemonCommand>) -> anyhow::Result<()> {
    let socket_path = find_socket()?;
    let _ = std::fs::remove_file(&socket_path); // clean up stale socket
    let listener = UnixListener::bind(socket_path)?;

    loop {
        let (mut stream, _) = listener.accept().await?;
        let mut buf = String::new();
        stream.read_to_string(&mut buf).await?;
        let _cmd = match buf.trim() {
            "next" => {
                let _ = tx.send(DaemonCommand::Next).await;
                continue;
            }
            "pause" => {
                let _ = tx.send(DaemonCommand::Pause).await;
                continue;
            }
            "resume" => {
                let _ = tx.send(DaemonCommand::Resume).await;
                continue;
            }
            "status" => {
                let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
                let _ = tx.send(DaemonCommand::Status(reply_tx)).await;
                if let Ok(reply) = reply_rx.await {
                    stream.write_all(reply.as_bytes()).await?;
                }
                continue;
            }
            other => {
                log::warn!("Unknown IPC command: {other}");
                continue;
            }
        };
    }
}
mod render;

// Re-exporting for use in `oneshot_mode()` in `main.rs`
pub use render::awww::detect_awww_binary;
pub use render::awww::ensure_awww_daemon;
use xdg::{BaseDirectories, BaseDirectoriesError};

/// Runs the persistent background process that cycles wallpapers and themes.
///
/// The daemon performs the following:
/// 1. Initializes the wallpaper cache and determines the rotation frequency.
/// 2. Sets up a listener for `SIGUSR1` to allow manual skips.
/// 3. Enters an infinite loop that updates themes and wallpapers based on the timer.
pub async fn run_loop<B: Backend>(config: Config, backend: B) -> anyhow::Result<()> {
    // Ensure the fallback theme is present before the first rotation
    crate::theme::ensure_theme_exists()?;

    let cache = WallpaperCache::new(&config.wallpaper_dir)?;

    // Parse the human-readable duration (e.g., "30m", "1h") into a Duration object
    let period: Duration =
        humantime::parse_duration(config.time.as_ref().expect("daemon mode requires --time"))
            .map_err(|e| anyhow::anyhow!("invalid duration: {e}"))?;

    // Initialize the chosen rendering engine (swaybg or awww)
    let mut renderer = render::Renderer::new(&config).await?;

    let (cmd_tx, mut cmd_rx) = tokio::sync::mpsc::channel::<DaemonCommand>(8);

    tokio::spawn(listen_for_ipc(cmd_tx));

    // Set up a signal listener for SIGUSR1 (allows users to run `pkill -USR1 randpaper`)
    let mut sig_usr1 = signal(SignalKind::user_defined1())?;

    let mut paused = false;

    loop {
        // Fetch active monitors; if the compositor is temporarily unreachable,
        // wait 5 seconds and retry rather than crashing the daemon.
        let monitors = match backend.get_active_monitors().await {
            Ok(m) => m,
            Err(e) => {
                log::error!("Failed to get monitors: {e}. Retrying in 5s...");
                sleep(Duration::from_secs(5)).await;
                continue;
            }
        };

        // Select a random wallpaper and update system-wide theme colors
        let img = cache.pick_random();
        let _ = update_theme_file(img);

        if !paused {
            let img = cache.pick_random();
            let _ = update_theme_file(img);
            renderer.apply(&config, &cache, &monitors).await?;
        }

        // Dispatch the wallpaper update to the specific renderer
        renderer.apply(&config, &cache, &monitors).await?;

        // The core wait logic:
        // Either wait for the full 'period' duration, OR
        // break out early if a SIGUSR1 is received.
        tokio::select! {
                () = sleep(period) => {}
                _ = sig_usr1.recv() => {
                    log::info!("Received skip signal (SIGUSR1). Cycling wallpaper immediately.");
                }
                Some(cmd) = cmd_rx.recv() => {
                    match cmd {
                        DaemonCommand::Next => {
                            log::info!("IPC next wallpaper");
                        }
                        DaemonCommand::Pause => {
                            log::info!("IPC pausing");
                            paused = true;
                        }
                        DaemonCommand::Resume => {
                            log::info!("IPC resuming");
                            paused = false;
                        }
        DaemonCommand::Status(reply) => {
            let msg = format!("running, paused={paused}");
            let _ = reply.send(msg);
        }

                        }
                    }
                }
    }
}
