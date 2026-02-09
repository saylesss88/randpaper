use anyhow::Result;
use async_trait::async_trait;

#[async_trait]
pub trait Backend {
    /// Returns a list of active monitor identifiers (e.g., "HDMI-A-1", "eDP-1")
    async fn get_active_monitors(&self) -> Result<Vec<String>>;
}
