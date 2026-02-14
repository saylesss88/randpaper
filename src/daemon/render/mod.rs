use crate::cli::{Cli, RendererType};
use crate::wallpaper::WallpaperCache;
use tokio::process::Child;

mod swaybg;
pub mod swww;

pub struct Renderer {
    swaybg_child: Option<Child>,
    swww_bin: Option<String>,
}

impl Renderer {
    pub async fn new(cli: &Cli) -> anyhow::Result<Self> {
        let swww_bin = match cli.renderer {
            RendererType::Swww => {
                let bin = swww::detect_swww_binary().await;
                swww::ensure_swww_daemon(&bin).await?;
                Some(bin)
            }
            RendererType::Swaybg => None,
        };
        Ok(Self {
            swaybg_child: None,
            swww_bin,
        })
    }

    pub async fn apply(
        &mut self,
        cli: &Cli,
        cache: &WallpaperCache,
        monitors: &[String],
    ) -> anyhow::Result<()> {
        match cli.renderer {
            RendererType::Swaybg => swaybg::apply(cache, monitors, &mut self.swaybg_child).await,
            RendererType::Swww => {
                let bin = self.swww_bin.as_deref().expect("Renderer::new sets this");
                swww::apply(cli, cache, monitors, bin).await
            }
        }
    }
}
