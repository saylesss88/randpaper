use rand::prelude::*;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Picks a random wallpaper file from `dir`.
///
/// # Errors
///
/// Returns an error if:
/// - the directory cannot be read, or
/// - no supported images (jpg, jpeg, png, bmp) are found in `dir`.
pub fn pick_random_wallpaper<P: AsRef<Path>>(dir: P) -> anyhow::Result<PathBuf> {
    let mut images = Vec::new();
    for entry in WalkDir::new(dir.as_ref()) {
        let entry = entry?;
        if entry.file_type().is_file() {
            let ext = entry
                .path()
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_lowercase(); // Convert to lowercase first

            if ["jpg", "jpeg", "png", "bmp"].contains(&ext.as_str()) {
                images.push(entry.path().to_path_buf());
            }
        }
    }
    if images.is_empty() {
        anyhow::bail!("no images found in {:?}", dir.as_ref().display());
    }

    let mut rng = rand::rng();
    let i = rng.random_range(0..images.len());
    Ok(images[i].clone())
}
