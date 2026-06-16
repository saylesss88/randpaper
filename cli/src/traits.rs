use anyhow::Result;
use async_trait::async_trait;

/// Defines the interface for interacting with different Window Managers or Compositors.
///
/// This trait allows the core logic to remain agnostic of the underlying display protocol
/// (e.g., Wayland vs. X11) or specific compositor implementations (e.g., Hyprland vs. Sway).
#[async_trait]
pub trait Backend {
    /// Returns a list of unique identifiers for monitors that are currently powered on and active.
    ///
    /// These identifiers (e.g., "HDMI-A-1", "eDP-1") are typically used to target
    /// wallpaper updates or coordinate-specific configuration changes.
    ///
    /// # Errors
    ///
    /// Returns an error if the backend fails to communicate with the compositor
    /// or if the monitor list cannot be parsed.
    async fn get_active_monitors(&self) -> Result<Vec<String>>;
}
