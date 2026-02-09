use rand::prelude::IndexedRandom;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub struct WallpaperCache {
    files: Vec<PathBuf>,
}

impl WallpaperCache {
    /// Scans the directory once and stores valid image paths in memory.
    pub fn new<P: AsRef<Path>>(dir: P) -> anyhow::Result<Self> {
        let mut files = Vec::new();
        let valid_exts = ["jpg", "jpeg", "png", "bmp", "webp"];

        for entry in WalkDir::new(dir) {
            let entry = entry?;
            let path = entry.path();
            // 1. Skip if it's not a file
            if !entry.file_type().is_file() {
                continue;
            }
            // 2. Skip if it has no valid extension (The "let-else" trick)
            let Some(ext) = path.extension().and_then(|s| s.to_str()) else {
                continue;
            };
            // 3. Final check and push
            if valid_exts.contains(&ext.to_lowercase().as_str()) {
                files.push(path.to_path_buf());
            }
        }

        if files.is_empty() {
            anyhow::bail!("No supported images found in directory.");
        }

        log::info!("Cached {} wallpapers.", files.len());
        Ok(Self { files })
    }

    /// Returns a random wallpaper from the cache.
    pub fn pick_random(&self) -> &PathBuf {
        let mut rng = rand::rng();
        self.files.choose(&mut rng).unwrap()
    }
}
