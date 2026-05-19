use crate::traits::Backend;
use anyhow::Context;
use async_trait::async_trait;
use tokio::process::Command;

/// A backend implementation for the `MangoWM` compositor.
///
/// Uses the `mmsg -O` command to list active output names.
pub struct MangoBackend;

#[async_trait]
impl Backend for MangoBackend {
    /// Retrieves a list of currently active monitor names via `mmsg`.
    ///
    /// Executes `mmsg -O` which prints one output name per line.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// * The `mmsg` binary is not found or fails to execute.
    /// * The output is not valid UTF-8.
    /// * No outputs are returned.
    async fn get_active_monitors(&self) -> anyhow::Result<Vec<String>> {
        let out = Command::new("mmsg")
            .arg("-O")
            .output()
            .await
            .context("failed to execute `mmsg`. Is MangoWM running?")?;

        let stdout = String::from_utf8(out.stdout).context("mmsg output was not valid UTF-8")?;

        let monitors: Vec<String> = stdout
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect();

        if monitors.is_empty() {
            anyhow::bail!("mmsg returned no active outputs");
        }

        Ok(monitors)
    }
}
