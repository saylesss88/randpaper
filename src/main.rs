#![allow(clippy::multiple_crate_versions)]
mod cli;
mod core;
mod hyprland;
mod sway;

use clap::Parser;

use crate::cli::{Backend, Cli};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.backend {
        Backend::Sway => sway::run_sway_loop(cli).await,
        Backend::Hyprpaper => hyprland::run_hyprpaper_loop(cli).await,
    }
}
