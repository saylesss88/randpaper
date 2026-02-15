use crate::traits::Backend;
use anyhow::Context;
use async_trait::async_trait;
use serde::Deserialize;
use tokio::process::Command;

/// A backend implementation for the Hyprland compositor.
///
/// This backend uses the `hyprctl` command-line utility to communicate with
/// the running Hyprland instance.
pub struct HyprlandBackend;

/// Represents a subset of the JSON data returned by `hyprctl monitors`.
#[derive(Debug, Deserialize)]
struct HyprMonitor {
    /// The name of the output (e.g., "eDP-1", "DP-2").
    name: String,
}

#[async_trait]
impl Backend for HyprlandBackend {
    /// Retrieves a list of currently active monitor names via `hyprctl`.
    ///
    /// It executes `hyprctl -j monitors` to get a JSON representation of the
    /// display layout. Because `hyprctl` may occasionally prepend non-JSON
    /// log messages, this method includes a cleaning step to find the start
    /// of the JSON array.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// * The `hyprctl` binary is not found or fails to execute.
    /// * The output from `hyprctl` is not valid UTF-8.
    /// * The JSON cannot be parsed into the expected format.    
    async fn get_active_monitors(&self) -> anyhow::Result<Vec<String>> {
        let out = Command::new("hyprctl")
            .args(["-j", "monitors"])
            .output()
            .await
            .context("failed to execute `hyprctl`. Is Hyprland running?")?;

        let raw = String::from_utf8(out.stdout)?;
        // Simple JSON cleaning logic
        // Find the first occurrence of '[' to skip any potential
        // debug logging or headers emitted by `hyprctl`.
        let start = raw.find('[').unwrap_or(0);
        let clean_json = &raw[start..];

        let monitors: Vec<HyprMonitor> = serde_json::from_str(clean_json)?;
        Ok(monitors.into_iter().map(|m| m.name).collect())
    }
}
