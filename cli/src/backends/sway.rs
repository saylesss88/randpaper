// Robust Sway backend:
// - Uses swayipc_async first (pure Rust IPC).
// - If it errors or times out, falls back to `swaymsg -t get_outputs -r`.
// - Never mutates SWAYSOCK env (avoids global races).
use crate::traits::Backend;
use anyhow::{Context, bail};
use async_trait::async_trait;
use serde::Deserialize;
use swayipc_async::Connection;
use tokio::process::Command;
use tokio::time::{Duration, timeout};

/// Sway backend implementation.
///
/// Can optionally accept a list of overridden outputs (from CLI args)
/// to ignore what Sway reports and force specific monitors.
pub struct SwayBackend {
    pub outputs_override: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct SwaymsgOutput {
    name: String,
    active: bool,
}

async fn get_outputs_via_swayipc() -> anyhow::Result<Vec<String>> {
    // Keep these short so oneshot never "hangs for a while".
    let mut conn = timeout(Duration::from_millis(300), Connection::new())
        .await
        .context("sway ipc: connect timed out")?
        .context("sway ipc: connect failed")?;

    let outputs = timeout(Duration::from_millis(300), conn.get_outputs())
        .await
        .context("sway ipc: get_outputs timed out")?
        .context("sway ipc: get_outputs failed")?;

    Ok(outputs
        .into_iter()
        .filter(|o| o.active)
        .map(|o| o.name)
        .collect())
}

async fn get_outputs_via_swaymsg() -> anyhow::Result<Vec<String>> {
    let out = timeout(
        Duration::from_secs(1),
        Command::new("swaymsg")
            .args(["-t", "get_outputs", "-r"])
            .output(),
    )
    .await
    .context("swaymsg: timed out")?
    .context("swaymsg: failed to spawn")?;

    if !out.status.success() {
        bail!(
            "swaymsg get_outputs failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }

    let outputs: Vec<SwaymsgOutput> =
        serde_json::from_slice(&out.stdout).context("swaymsg: invalid JSON")?;

    Ok(outputs
        .into_iter()
        .filter(|o| o.active)
        .map(|o| o.name)
        .collect())
}

#[async_trait]
impl Backend for SwayBackend {
    async fn get_active_monitors(&self) -> anyhow::Result<Vec<String>> {
        // If the user manually specified outputs in CLI, skip querying Sway
        if !self.outputs_override.is_empty() {
            return Ok(self.outputs_override.clone());
        }

        // First try: pure Rust IPC
        match get_outputs_via_swayipc().await {
            Ok(names) if !names.is_empty() => return Ok(names),
            Ok(_) => {
                // empty list is suspicious; fall through to swaymsg
                log::warn!("sway ipc returned 0 active outputs; falling back to swaymsg");
            }
            Err(e) => {
                log::warn!("sway ipc failed ({e:#}); falling back to swaymsg");
            }
        }

        // Fallback: swaymsg
        get_outputs_via_swaymsg().await
    }
}
