use crate::cli::Cli;
use crate::wallpaper::WallpaperCache;
use anyhow::Context;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::sleep;

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

async fn swww_ready(swww_bin: &str) -> bool {
    (Command::new(swww_bin).arg("query").status().await).is_ok_and(|st| st.success())
}

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

pub async fn apply(
    cli: &Cli,
    cache: &WallpaperCache,
    monitors: &[String],
    swww_bin: &str,
) -> anyhow::Result<()> {
    let step = cli.transition_step.to_string();
    let fps = cli.transition_fps.to_string();

    for monitor in monitors {
        let img = cache.pick_random();

        let out = Command::new(swww_bin)
            .arg("img")
            .arg(img)
            .arg("-o")
            .arg(monitor)
            .arg("--transition-type")
            .arg(&cli.transition_type)
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
