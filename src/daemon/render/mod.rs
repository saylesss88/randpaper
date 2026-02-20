use crate::cli::{Config, RendererType};
use crate::wallpaper::WallpaperCache;
use tokio::process::Child;

pub mod awww;
mod swaybg;

/// Manages the lifecycle and execution of wallpaper rendering backends.
///
/// The `Renderer` abstracts over different wallpaper utilities (like `awww` or `swaybg`),
/// handling process management for long-running children and binary detection.
pub struct Renderer {
    /// Holds a reference to the active `swaybg` process, if running.
    /// This allows the renderer to kill the old process before starting a new one.
    swaybg_child: Option<Child>,
    /// The path to the detected `awww` binary.
    awww_bin: Option<String>,
}

impl Renderer {
    /// Creates a new `Renderer` instance based on the user's CLI configuration.
    ///
    /// If the `Awww` renderer is selected, this method will:
    /// 1. Detect the `awww` binary in the system path.
    /// 2. Ensure the `awww` daemon is initialized and running.
    ///
    /// # Errors
    ///
    /// Returns an error if the renderer initialization (e.g., starting the daemon) fails.
    pub async fn new(config: &Config) -> anyhow::Result<Self> {
        let awww_bin = match config.renderer {
            RendererType::Awww => {
                let bin = awww::detect_awww_binary().await;
                awww::ensure_awww_daemon(&bin).await?;
                Some(bin)
            }
            RendererType::Swaybg => None,
        };
        Ok(Self {
            swaybg_child: None,
            awww_bin,
        })
    }

    /// Applies the current wallpaper configuration to the specified monitors.
    ///
    /// This method routes the request to the appropriate backend module based on
    /// the `RendererType` provided in the CLI arguments.
    ///
    /// # Arguments
    ///
    /// * `cli` - The global command-line configuration.
    /// * `cache` - The cache containing the wallpaper images to be displayed.
    /// * `monitors` - A list of monitor names (e.g., "eDP-1", "DP-2") to update.
    ///
    /// # Panics
    ///
    /// Panics if the renderer is set to `Awww` but the binary path was never initialized.
    pub async fn apply(
        &mut self,
        config: &Config,
        cache: &WallpaperCache,
        monitors: &[String],
    ) -> anyhow::Result<()> {
        match config.renderer {
            RendererType::Swaybg => swaybg::apply(cache, monitors, &mut self.swaybg_child).await,
            RendererType::Awww => {
                let bin = self.awww_bin.as_deref().expect("Renderer::new sets this");
                awww::apply(config, cache, monitors, bin).await
            }
        }
    }
}
