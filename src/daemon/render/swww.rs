use crate::cli::Config;
use crate::wallpaper::WallpaperCache;
use anyhow::Context;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::sleep;

/// Attempts to find a compatible binary for `swww`.
///
/// Checks for `swww` first, then `awww` (a common wrapper/alternative).
/// Defaults to "swww" if neither are found.
pub async fn detect_swww_binary() -> String {
    if Command::new("swww").arg("--help").output().await.is_ok() {
        return "swww".to_string();
    }
    if Command::new("awww").arg("--help").output().await.is_ok() {
        return "awww".to_string();
    }
    log::warn!("Neither 'swww' nor 'awww' found. Defaulting to 'swww'.");
    "swww".to_string()
}

/// Checks if the `swww-daemon` is currently intitialized and responding to queries.
async fn swww_ready(swww_bin: &str) -> bool {
    (Command::new(swww_bin).arg("query").status().await).is_ok_and(|st| st.success())
}

/// Ensures the `swww-daemon` is running.
///
/// If the daemon is not responsive, it checks for an existing process via `pgrep`.
/// If no process is found, it spawns a new daemon and waits briefly for it to initialize.
pub async fn ensure_swww_daemon(swww_bin: &str) -> anyhow::Result<()> {
    if swww_ready(swww_bin).await {
        return Ok(());
    }

    let daemon_name = format!("{swww_bin}-daemon");
    let status = Command::new("pgrep")
        .arg("-x")
        .arg(&daemon_name)
        .status()
        .await;

    match status {
        Ok(es) if es.success() => log::info!("{daemon_name} is already running"),
        _ => {
            log::info!("Starting {daemon_name}...");
            Command::new(&daemon_name)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
                .with_context(|| format!("failed to spawn {daemon_name}"))?;

            sleep(Duration::from_millis(500)).await;
        }
    }

    Ok(())
}

/// Sends commands to the `swww` daemon to update wallpapers with transitions.
///
/// This loops through each monitor and calls the `img` command.
/// It uses transition settings (type, step, fps) provided in the `Cli` config.
///
/// # Errors
/// Returns an error if the binary cannot be executed or if `swww` returns a non-zero exit code.
pub async fn apply(
    config: &Config,
    cache: &WallpaperCache,
    monitors: &[String],
    swww_bin: &str,
) -> anyhow::Result<()> {
    let step = config.transition_step.to_string();
    let fps = config.transition_fps.to_string();

    for monitor in monitors {
        let img = cache.pick_random();

        let out = Command::new(swww_bin)
            .arg("img")
            .arg(img)
            .arg("-o")
            .arg(monitor)
            .arg("--transition-type")
            .arg(&config.transition_type)
            .arg("--transition-step")
            .arg(&step)
            .arg("--transition-fps")
            .arg(&fps)
            .output()
            .await
            .with_context(|| format!("failed to run {swww_bin}"))?;

        if !out.status.success() {
            anyhow::bail!(
                "{swww_bin} failed: {}",
                String::from_utf8_lossy(&out.stderr)
            );
        }
    }

    Ok(())
}
