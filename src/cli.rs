use clap::{Parser, ValueEnum};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about = "A random wallpaper setter daemon for Hyprland and Sway", long_about = None)]
pub struct Cli {
    /// Directory containing wallpapers
    #[arg(default_value = ".")]
    pub wallpaper_dir: PathBuf,

    /// Time interval for wallpaper updates (e.g., "30m", "1h", "300s")
    #[arg(short, long, default_value = "30m")]
    pub time: String,

    /// Which backend to use (hyprland or sway)
    #[arg(short, long, value_enum, default_value_t = BackendType::Sway)]
    pub backend: BackendType,

    /// Optional: Force specific outputs for Sway (ignored by Hyprland backend)
    #[arg(short, long)]
    pub outputs: Vec<String>,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
pub enum BackendType {
    /// Use the Hyprland IPC (hyprctl)
    Hyprland,
    /// Use the Sway IPC
    Sway,
}
