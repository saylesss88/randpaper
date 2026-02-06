use super::core::pick_random_wallpaper;
use crate::cli::Cli;
use std::time::Duration;
use swayipc_async::Connection;
use swayipc_types::Output;
use tokio::process::Command;
use tokio::time::sleep;

/// Runs the wallpaper loop for Sway, changing backgrounds on all active outputs.
///
/// # Errors
///
/// Returns an error if:
/// - swayipc connection fails,
/// - `swaybg` cannot be started or killed, or
/// - the wallpaper directory cannot be read.
pub async fn run_sway_loop(cli: Cli) -> anyhow::Result<()> {
    let period: Duration = parse_duration::parse(&cli.time)
        .map_err(|e| anyhow::anyhow!("invalid duration '{}': {e}", cli.time))?;

    let mut conn = Connection::new().await?;
    let sway_outputs: Vec<Output> = conn.get_outputs().await?;

    let outputs: Vec<String> = if cli.outputs.is_empty() {
        sway_outputs
            .iter()
            .filter(|o| o.active)
            .map(|o| o.name.clone())
            .collect()
    } else {
        cli.outputs
    };

    log::info!("using outputs: {outputs:?}");

    loop {
        let mut args = Vec::new();
        for output in &outputs {
            let img_path = pick_random_wallpaper(&cli.wallpaper_dir)?;
            let img = img_path.display().to_string();

            args.push("-o".to_string());
            args.push(output.clone());
            args.push("-m".to_string());
            args.push("fill".to_string());
            args.push("-i".to_string());
            args.push(img);
        }

        Command::new("pkill").arg("swaybg").status().await?;
        Command::new("swaybg").args(&args).status().await?;

        sleep(period).await;
    }
}
