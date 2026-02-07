#![allow(clippy::multiple_crate_versions)]
use randpaper::cli::{Backend, Cli};
use randpaper::{hyprland, sway};

use clap::Parser;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.backend {
        Backend::Sway => sway::run_sway_loop(cli).await,
        Backend::Hyprpaper => hyprland::run_hyprpaper_loop(cli).await,
    }
}
