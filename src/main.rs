#![allow(clippy::multiple_crate_versions)]
use clap::Parser;
use std::path::{Path, PathBuf};
use std::time::Duration;
use swayipc_async::Connection;
use swayipc_types::Output;
use tokio::process::Command;
use tokio::time::sleep;

// Hyprland dependencies
use hyprland::shared::HyprData;

/// Pick a random wallpaper file from `dir`.
fn pick_random_wallpaper<P: AsRef<Path>>(dir: P) -> anyhow::Result<PathBuf> {
    let mut images = Vec::new();
    for entry in walkdir::WalkDir::new(dir.as_ref()) {
        let entry = entry?;
        if entry.file_type().is_file() {
            let ext = entry.path().extension().unwrap_or_default();
            if ["jpg", "jpeg", "png", "bmp"].contains(&ext.to_str().unwrap_or("")) {
                images.push(entry.path().to_path_buf());
            }
        }
    }
    if images.is_empty() {
        anyhow::bail!("no images found in {:?}", dir.as_ref().display());
    }

    let mut rng = rand::rng();
    let i = rand::Rng::random_range(&mut rng, 0..images.len());
    Ok(images[i].clone())
}

/// Time backend enum.
#[derive(Clone, Debug, clap::ValueEnum)]
enum Backend {
    Sway,
    Hyprpaper,
}

/// CLI args for randpaper.
#[derive(clap::Parser, Debug)]
#[clap(author, version, about)]
struct Cli {
    /// Directory containing wallpapers.
    wallpaper_dir: PathBuf,

    /// Time between wallpaper changes, e.g. "1h", "30m", "1d".
    #[arg(short, long, default_value = "30m")]
    time: String,

    /// Output names to target (default: auto‑discover).
    #[arg(short, long)]
    outputs: Vec<String>,

    /// Backend to use (Sway or Hyprpaper).
    #[arg(short, long, value_enum, default_value = "sway")]
    backend: Backend,
}

async fn run_sway_loop(cli: Cli) -> anyhow::Result<()> {
    let period: Duration = parse_duration::parse(&cli.time)
        .map_err(|e| anyhow::anyhow!("invalid duration '{}': {e}", cli.time))?;

    let mut conn = Connection::new().await?;
    let sway_outputs: Vec<Output> = conn.get_outputs().await?;

    let outputs: Vec<String> = if cli.outputs.is_empty() {
        sway_outputs
            .iter()
            .filter(|o| o.active)
            .map(|o| o.name.clone())
            .collect()
    } else {
        cli.outputs
    };

    eprintln!("using outputs: {outputs:?}");

    loop {
        eprintln!("picking new wallpapers for outputs: {outputs:?}");

        let mut args = Vec::new();
        for output in &outputs {
            let img_path = pick_random_wallpaper(&cli.wallpaper_dir)?;
            let img = img_path.display().to_string();

            args.push("-o".to_string());
            args.push(output.clone());
            args.push("-m".to_string());
            args.push("fill".to_string());
            args.push("-i".to_string());
            args.push(img);
        }

        eprintln!("swaybg args: {args:#?}");

        Command::new("pkill").arg("swaybg").status().await?;
        Command::new("swaybg").args(&args).status().await?;

        sleep(period).await;
    }
}
use hyprland::data::Monitors;

async fn run_hyprpaper_loop(cli: Cli) -> anyhow::Result<()> {
    let period: Duration = parse_duration::parse(&cli.time)
        .map_err(|e| anyhow::anyhow!("invalid duration '{}': {e}", cli.time))?;

    // Ensure hyprpaper is running first (optional, but good practice)
    // You might want to spawn it if not running, or assume the user started it.
    // For now, we assume it's running or started by the user's config.

    loop {
        // 1. Get monitors using hyprland crate
        let monitors = Monitors::get()?;

        for monitor in monitors {
            let img = pick_random_wallpaper(&cli.wallpaper_dir)?;
            let img_path = img.to_string_lossy();
            let monitor_name = monitor.name;

            // 2. Preload the image first (hyprpaper requirement)
            Command::new("hyprctl")
                .arg("hyprpaper")
                .arg("preload")
                .arg(&*img_path)
                .status()
                .await?;

            // 3. Set the wallpaper
            Command::new("hyprctl")
                .arg("hyprpaper")
                .arg("wallpaper")
                .arg(format!("{monitor_name},{img_path}"))
                .status()
                .await?;

            // 4. Unload unused wallpapers to save RAM (optional but recommended)
            // Command::new("hyprctl").arg("hyprpaper").arg("unload").arg("all").status().await?;
        }

        sleep(period).await;
    }
}
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.backend {
        Backend::Sway => run_sway_loop(cli).await,
        Backend::Hyprpaper => run_hyprpaper_loop(cli).await,
    }
}
