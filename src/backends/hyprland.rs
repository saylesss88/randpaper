use crate::traits::Backend;
use anyhow::Context;
use async_trait::async_trait;
use serde::Deserialize;
use tokio::process::Command;

pub struct HyprlandBackend;

#[derive(Debug, Deserialize)]
struct HyprMonitor {
    name: String,
}

#[async_trait]
impl Backend for HyprlandBackend {
    async fn get_active_monitors(&self) -> anyhow::Result<Vec<String>> {
        let out = Command::new("hyprctl")
            .args(["-j", "monitors"])
            .output()
            .await
            .context("failed to run hyprctl")?;

        let raw = String::from_utf8(out.stdout)?;
        // Simple JSON cleaning logic
        let start = raw.find('[').unwrap_or(0);
        let clean_json = &raw[start..];

        let monitors: Vec<HyprMonitor> = serde_json::from_str(clean_json)?;
        Ok(monitors.into_iter().map(|m| m.name).collect())
    }
}
