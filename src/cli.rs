use clap::{Parser, ValueEnum};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Directory containing wallpapers
    #[arg(default_value = ".")]
    pub wallpaper_dir: PathBuf,

    /// Time interval for wallpaper updates
    #[arg(short, long, default_value = "30m")]
    pub time: String,

    /// Which backend to use for Monitor Detection
    #[arg(short, long, value_enum, default_value_t = BackendType::Sway)]
    pub backend: BackendType,

    /// Which tool to use to set the wallpaper
    #[arg(short, long, value_enum, default_value_t = RendererType::Swaybg)]
    pub renderer: RendererType,

    /// Optional: Force specific outputs for Sway
    #[arg(short, long)]
    pub outputs: Vec<String>,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
pub enum BackendType {
    Hyprland,
    Sway,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
pub enum RendererType {
    /// Uses 'swaybg' (Supports multiple monitors, no transitions)
    Swaybg,
    /// Uses 'swww' (Supports transitions, Hyprland specific)
    Swww,
    // Possibly add Hyprpaper in the future
}
