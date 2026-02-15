use crate::cli::Cli;
use crate::theme::update_theme_file;
use crate::traits::Backend;
use crate::wallpaper::WallpaperCache;

use std::time::Duration;
use tokio::signal::unix::{SignalKind, signal};
use tokio::time::sleep;

mod render;

// Re-exporting for use in `oneshot_mode()` in `main.rs`
pub use render::swww::detect_swww_binary;

/// Runs the persistent background process that cycles wallpapers and themes.
///
/// The daemon performs the following:
/// 1. Initializes the wallpaper cache and determines the rotation frequency.
/// 2. Sets up a listener for `SIGUSR1` to allow manual skips.
/// 3. Enters an infinite loop that updates themes and wallpapers based on the timer.
pub async fn run_loop<B: Backend>(cli: Cli, backend: B) -> anyhow::Result<()> {
    // Ensure the fallback theme is present before the first rotation
    crate::theme::ensure_theme_exists()?;

    let cache = WallpaperCache::new(&cli.wallpaper_dir)?;

    // Parse the human-readable duration (e.g., "30m", "1h") into a Duration object
    let period: Duration =
        parse_duration::parse(cli.time.as_ref().expect("daemon mode requires --time"))
            .map_err(|e| anyhow::anyhow!("invalid duration: {e}"))?;

    // Initialize the chosen rendering engine (swaybg or swww)
    let mut renderer = render::Renderer::new(&cli).await?;

    // if cli.renderer == RendererType::Swww {
    // optional: log renderer init already ensured daemon
    // }

    // Set up a signal listener for SIGUSR1 (allows users to run `pkill -USR1 randpaper`)
    let mut sig_usr1 = signal(SignalKind::user_defined1())?;

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

        // Dispatch the wallpaper update to the specific renderer
        renderer.apply(&cli, &cache, &monitors).await?;

        // The core wait logic:
        // Either wait for the full 'period' duration, OR
        // break out early if a SIGUSR1 is received.
        tokio::select! {
            () = sleep(period) => {}
            _ = sig_usr1.recv() => {
                log::info!("Received skip signal (SIGUSR1). Cycling wallpaper immediately.");
            }
        }
    }
}
