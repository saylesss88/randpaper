use super::core::pick_random_wallpaper;
use crate::cli::Cli;
use anyhow::Context;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::sleep;

async fn set_wallpaper(img_path: &str) -> anyhow::Result<()> {
    let status = Command::new("swww")
        .args([
            "img",
            img_path,
            "--transition-type",
            "fade",
            "--transition-duration",
            "1",
        ])
        .status()
        .await
        .context("failed to run swww")?;

    if !status.success() {
        anyhow::bail!("swww img command failed");
    }
    Ok(())
}

/// Runs the main loop for hyprpaper.
///
/// # Errors
///
/// This function will return an error if the hyprpaper process fails to start,
/// or if the communication channel is closed unexpectedly.
pub async fn run_hyprpaper_loop(cli: Cli) -> anyhow::Result<()> {
    let period: Duration = parse_duration::parse(&cli.time)
        .map_err(|e| anyhow::anyhow!("invalid duration '{}': {e}", cli.time))?;

    loop {
        let img = pick_random_wallpaper(&cli.wallpaper_dir)?;
        let img_abs = img
            .canonicalize()
            .with_context(|| format!("failed to canonicalize wallpaper path: {}", img.display()))?;

        set_wallpaper(&img_abs.to_string_lossy()).await?;
        sleep(period).await;
    }
}
