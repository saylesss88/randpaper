#![allow(clippy::multiple_crate_versions)]
mod backends;
mod cli;
mod daemon;
mod traits;
mod wallpaper;

use crate::backends::hyprland::HyprlandBackend;
use crate::backends::sway::SwayBackend;
use clap::Parser;
use cli::{BackendType, Cli}; // Assuming you have an enum in CLI for backend choice

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    let cli = Cli::parse();

    match cli.backend {
        BackendType::Hyprland => {
            log::info!("Using Hyprland backend");
            daemon::run_loop(cli, HyprlandBackend).await?;
        }
        BackendType::Sway => {
            log::info!("Using Sway backend");
            // Assuming SwayBackend::new() or struct init
            let backend = SwayBackend {
                outputs_override: cli.outputs.clone(),
            };
            daemon::run_loop(cli, backend).await?;
        }
    }

    Ok(())
}
