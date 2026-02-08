use super::core::pick_random_wallpaper;
use crate::cli::Cli;
use anyhow::Context;
use serde::Deserialize;
use std::{path::PathBuf, time::Duration};
use tokio::process::Command;
use tokio::time::sleep;

#[derive(Debug, Deserialize)]
struct HyprMonitor {
    name: String,
}

// Hyprctl JSON output has historically had some "not valid JSON" edge cases;
// this just finds the first '[' or '{' and tries to parse from there. [web:164]
fn json_payload(s: &str) -> &str {
    let start = s.find('[').or_else(|| s.find('{')).unwrap_or(0);
    &s[start..]
}

async fn hyprctl_json(args: &[&str]) -> anyhow::Result<String> {
    let out = Command::new("hyprctl")
        .args(args)
        .output()
        .await
        .with_context(|| format!("failed to run hyprctl {}", args.join(" ")))?;

    if !out.status.success() {
        anyhow::bail!(
            "hyprctl {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr)
        );
    }

    Ok(String::from_utf8(out.stdout).context("hyprctl output was not valid UTF-8")?)
}

async fn get_monitor_names() -> anyhow::Result<Vec<String>> {
    // Prefer: hyprctl -j monitors
    let raw = hyprctl_json(&["-j", "monitors"]).await?;
    let raw = json_payload(&raw);

    let monitors: Vec<HyprMonitor> =
        serde_json::from_str(raw).context("failed to parse `hyprctl -j monitors` JSON")?;

    Ok(monitors.into_iter().map(|m| m.name).collect())
}

pub async fn run_hyprpaper_loop(cli: Cli) -> anyhow::Result<()> {
    let period: Duration = parse_duration::parse(&cli.time)
        .map_err(|e| anyhow::anyhow!("invalid duration '{}': {e}", cli.time))?;

    loop {
        let monitor_names = get_monitor_names().await?;

        for monitor_name in monitor_names {
            let img: PathBuf = pick_random_wallpaper(&cli.wallpaper_dir)?;
            let img_abs = img
                .canonicalize()
                .with_context(|| format!("failed to canonicalize wallpaper path: {img:?}"))?;

            let img_path = img_abs.to_string_lossy();

            // `reload` loads + sets (+ effectively swaps) in one command.
            let arg = format!("{monitor_name},{img_path}");
            let status = Command::new("hyprctl")
                .args(["hyprpaper", "reload"])
                .arg(arg)
                .status()
                .await
                .context("failed to run `hyprctl hyprpaper reload`")?;

            if !status.success() {
                anyhow::bail!("`hyprctl hyprpaper reload` returned non-zero exit status");
            }
        }

        sleep(period).await;
    }
}
