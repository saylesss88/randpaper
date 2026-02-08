use rand::prelude::IndexedRandom;
use std::path::Path;
use walkdir::WalkDir;

/// Picks a random wallpaper file from `dir`.
///
/// # Errors
///
/// Returns an error if:
/// - the directory cannot be read, or
/// - no supported images are found in `dir`.
pub fn pick_random_wallpaper(dir: &Path) -> anyhow::Result<std::path::PathBuf> {
    let mut imgs = Vec::new();

    for ent in WalkDir::new(dir).follow_links(true) {
        let Ok(ent) = ent else { continue };

        let p = ent.path();

        if !p.is_file() {
            continue;
        }

        let ext = p
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();

        if matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "webp") {
            imgs.push(p.to_path_buf());
        }
    }

    let mut rng = rand::rng();
    imgs.choose(&mut rng)
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("no images found in {}", dir.display()))
}
