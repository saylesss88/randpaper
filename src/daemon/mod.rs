use crate::cli::{Cli, RendererType};
use crate::theme::update_theme_file;
use crate::traits::Backend;
use crate::wallpaper::WallpaperCache;

use std::time::Duration;
use tokio::signal::unix::{SignalKind, signal};
use tokio::time::sleep;

mod render;

// Keep the old path working for oneshot_mode()
pub use render::swww::detect_swww_binary;

pub async fn run_loop<B: Backend>(cli: Cli, backend: B) -> anyhow::Result<()> {
    crate::theme::ensure_theme_exists()?;

    let cache = WallpaperCache::new(&cli.wallpaper_dir)?;
    let period: Duration =
        parse_duration::parse(cli.time.as_ref().expect("daemon mode requires --time"))
            .map_err(|e| anyhow::anyhow!("invalid duration: {e}"))?;

    let mut renderer = render::Renderer::new(&cli).await?;

    if cli.renderer == RendererType::Swww {
        // optional: log renderer init already ensured daemon
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

        renderer.apply(&cli, &cache, &monitors).await?;

        tokio::select! {
            () = sleep(period) => {}
            _ = sig_usr1.recv() => {
                log::info!("Received skip signal (SIGUSR1). Cycling wallpaper immediately.");
            }
        }
    }
}
