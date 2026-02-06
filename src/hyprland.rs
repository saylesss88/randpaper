use super::core::pick_random_wallpaper;
use crate::cli::Cli;
use hyprland::data::Monitors;
use hyprland::shared::HyprData;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::sleep;

/// Runs the wallpaper loop for Hyprland, changing backgrounds on all monitors.
///
/// # Errors
///
/// Returns an error if:
/// - the Hyprland IPC socket cannot be reached,
/// - `hyprctl` / `hyprpaper` commands fail, or
/// - the wallpaper directory cannot be read.
pub async fn run_hyprpaper_loop(cli: Cli) -> anyhow::Result<()> {
    let period: Duration = parse_duration::parse(&cli.time)
        .map_err(|e| anyhow::anyhow!("invalid duration '{}': {e}", cli.time))?;

    loop {
        let monitors = Monitors::get()?;

        for monitor in monitors {
            let img = pick_random_wallpaper(&cli.wallpaper_dir)?;
            let img_path = img.to_string_lossy();
            let monitor_name = monitor.name;

            Command::new("hyprctl")
                .arg("hyprpaper")
                .arg("preload")
                .arg(&*img_path)
                .status()
                .await?;

            Command::new("hyprctl")
                .arg("hyprpaper")
                .arg("wallpaper")
                .arg(format!("{monitor_name},{img_path}"))
                .status()
                .await?;

            Command::new("hyprctl")
                .arg("hyprpaper")
                .arg("unload")
                .arg("all")
                .status()
                .await?;
        }

        sleep(period).await;
    }
}
