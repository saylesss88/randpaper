use crate::traits::Backend;
use async_trait::async_trait;
use swayipc_async::Connection;

/// Sway backend implementation.
///
/// Can optionally accept a list of overridden outputs (from CLI args)
/// to ignore what Sway reports and force specific monitors.
pub struct SwayBackend {
    pub outputs_override: Vec<String>,
}

#[async_trait]
impl Backend for SwayBackend {
    async fn get_active_monitors(&self) -> anyhow::Result<Vec<String>> {
        // If the user manually specified outputs in CLI, skip querying Sway
        if !self.outputs_override.is_empty() {
            return Ok(self.outputs_override.clone());
        }

        // Connect to Sway IPC
        let mut conn = Connection::new().await?;
        let outputs = conn.get_outputs().await?;

        // Filter for active outputs only
        let monitor_names = outputs
            .into_iter()
            .filter(|o| o.active)
            .map(|o| o.name)
            .collect();

        Ok(monitor_names)
    }
}
