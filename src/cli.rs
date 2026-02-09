use std::path::PathBuf;

/// Time backend enum.
#[derive(Clone, Debug, clap::ValueEnum)]
pub enum Backend {
    Sway,
    Hyprland,
}

/// A lightweight Wayland wallpaper daemon that randomizes backgrounds per‑screen.
#[derive(clap::Parser, Debug)]
#[clap(author, version, about)]
pub struct Cli {
    /// Directory containing wallpapers (supports jpg, png, bmp).
    pub wallpaper_dir: PathBuf,

    /// Time between wallpaper changes, (e.g. "1h", "30m", "1d").
    #[arg(short, long, default_value = "30m")]
    pub time: String,

    /// Output names to target (default: auto‑discover).
    #[arg(short, long)]
    pub outputs: Vec<String>,

    /// Backend to use (Sway or Hyprland).
    #[arg(short, long, value_enum, default_value = "sway")]
    pub backend: Backend,
}
