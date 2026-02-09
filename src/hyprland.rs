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
// this just finds the first '[' or '{' and tries to parse from there.
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

    String::from_utf8(out.stdout).context("hyprctl output was not valid UTF-8")
}

async fn get_monitor_names() -> anyhow::Result<Vec<String>> {
    let raw = hyprctl_json(&["-j", "monitors"]).await?;
    let raw = json_payload(&raw);

    let monitors: Vec<HyprMonitor> =
        serde_json::from_str(raw).context("failed to parse `hyprctl -j monitors` JSON")?;

    Ok(monitors.into_iter().map(|m| m.name).collect())
}

/// Runs the wallpaper loop using `swaybg`, changing backgrounds on all active outputs.
///
/// This function repeatedly:
/// - queries active monitors via `hyprctl -j monitors`,
/// - picks a random wallpaper for each monitor from `cli.wallpaper_dir`,
/// - kills any running `swaybg`, then restarts it with the new images.
///
/// # Errors
///
/// Returns an error if:
/// - `hyprctl` fails to list monitors or its output is not valid UTF‑8,
/// - the monitor JSON cannot be parsed,
/// - a wallpaper path cannot be canonicalized or read,
/// - `pkill swaybg` fails, or
/// - `swaybg` cannot be started or exits with a non‑zero status.
pub async fn run_swaybg_loop(cli: Cli) -> anyhow::Result<()> {
    let period: Duration = parse_duration::parse(&cli.time)
        .map_err(|e| anyhow::anyhow!("invalid duration '{}': {e}", cli.time))?;

    loop {
        let monitor_names = get_monitor_names().await?;

        let mut args = Vec::new();

        for monitor_name in monitor_names {
            let img: PathBuf = pick_random_wallpaper(&cli.wallpaper_dir)?;
            let img_abs = img.canonicalize().with_context(|| {
                format!("failed to canonicalize wallpaper path: {}", img.display())
            })?;
            let img_path = img_abs.to_string_lossy();

            args.push("-o".to_owned());
            args.push(monitor_name);
            args.push("-m".to_owned());
            args.push("fill".to_owned());
            args.push("-i".to_owned());
            args.push(img_path.to_string());
        }

        // Kill any existing swaybg
        Command::new("pkill")
            .arg("swaybg")
            .status()
            .await
            .with_context(|| "failed to kill existing swaybg")?;

        // Start swaybg with the built args
        if !args.is_empty() {
            let status = Command::new("swaybg")
                .args(&args)
                .status()
                .await
                .with_context(|| "failed to start swaybg")?;

            if !status.success() {
                anyhow::bail!("swaybg exited with non-zero status");
            }
            // we intentionally let swaybg run in the background; next loop iter
            // will kill and restart it
        }

        sleep(period).await;
    }
}
