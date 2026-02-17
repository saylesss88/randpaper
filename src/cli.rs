use clap::{Parser, ValueEnum};
use figment::{
    Figment,
    providers::{Env, Format, Serialized, Toml},
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Random Photo from Pexels :`city_sunset`
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Directory containing wallpapers
    #[arg(long)]
    pub wallpaper_dir: Option<PathBuf>,

    /// Time interval for wallpaper updates (e.g., "5m", "1h")
    #[arg(short, long)]
    pub time: Option<String>,

    /// Which backend to use for Monitor Detection
    #[arg(short, long, value_enum)]
    pub backend: Option<BackendType>,

    /// Which tool to use to set the wallpaper
    #[arg(short, long, value_enum)]
    pub renderer: Option<RendererType>,

    /// Optional: Force specific outputs for Sway
    #[arg(short, long)]
    pub outputs: Option<Vec<String>>,

    #[arg(short, long)]
    pub transition_type: Option<String>,

    #[arg(short = 's', long)]
    pub transition_step: Option<u8>,

    #[arg(short = 'f', long)]
    pub transition_fps: Option<u8>,

    /// Path to config file
    #[arg(long)]
    pub config: Option<PathBuf>,
}

// Merge CLI Overrides
// We manually map CLI args to the config structure to ensure CLI always wins
// but only if the user actually provided the flag (Option::Some).
#[derive(Serialize)]
struct CliOverrides {
    #[serde(skip_serializing_if = "Option::is_none")]
    wallpaper_dir: Option<PathBuf>,

    #[serde(skip_serializing_if = "Option::is_none")]
    time: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    backend: Option<BackendType>,

    #[serde(skip_serializing_if = "Option::is_none")]
    renderer: Option<RendererType>,

    #[serde(skip_serializing_if = "Option::is_none")]
    outputs: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    transition_type: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    transition_step: Option<u8>,

    #[serde(skip_serializing_if = "Option::is_none")]
    transition_fps: Option<u8>,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BackendType {
    Hyprland,
    Sway,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RendererType {
    Swaybg,
    #[clap(alias = "awww")]
    Swww,
}

/// The final configuration used by the application
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Config {
    pub wallpaper_dir: PathBuf,
    pub time: Option<String>,
    pub backend: BackendType,
    pub renderer: RendererType,
    pub outputs: Vec<String>,
    pub transition_type: String,
    pub transition_step: u8,
    pub transition_fps: u8,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            wallpaper_dir: PathBuf::from("."),
            time: None,
            backend: BackendType::Sway,
            renderer: RendererType::Swaybg,
            outputs: Vec::new(),
            transition_type: "simple".to_string(),
            transition_step: 90,
            transition_fps: 30,
        }
    }
}

impl Config {
    pub fn new() -> anyhow::Result<Self> {
        let cli = Cli::parse();

        let mut builder = Figment::new().merge(Serialized::defaults(Self::default()));

        // 1. Determine config file path
        // Priority: CLI arg > XDG Config > None
        let config_file = cli.config.or_else(|| {
            // with_prefix returns BaseDirectories directly in your version
            let xdg_dirs = xdg::BaseDirectories::with_prefix("randpaper");
            xdg_dirs.find_config_file("config.toml")
        });
        // 2. Load Config File if found
        if let Some(path) = config_file {
            builder = builder.merge(Toml::file(path));
        }

        // 3. CLI overrides
        let overrides = CliOverrides {
            wallpaper_dir: cli.wallpaper_dir,
            time: cli.time,
            backend: cli.backend,
            renderer: cli.renderer,
            outputs: cli.outputs,
            transition_type: cli.transition_type,
            transition_step: cli.transition_step,
            transition_fps: cli.transition_fps,
        };

        builder = builder.merge(Serialized::defaults(overrides));

        // 4. Merge Environment Variables (optional, but good practice)
        builder = builder.merge(Env::prefixed("RANDPAPER_"));

        Ok(builder.extract()?)
    }
}
