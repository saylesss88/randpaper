use rand::prelude::IndexedRandom;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub struct WallpaperCache {
    files: Vec<PathBuf>, // store absolute/canonical paths
}

impl WallpaperCache {
    pub fn new<P: AsRef<Path>>(dir: P) -> anyhow::Result<Self> {
        let mut files = Vec::new();

        for entry in WalkDir::new(dir) {
            let entry = entry?;
            if !entry.file_type().is_file() {
                continue;
            }

            let path = entry.path();

            let Some(ext) = path.extension().and_then(|s| s.to_str()) else {
                continue;
            };

            let is_supported = ext.eq_ignore_ascii_case("jpg")
                || ext.eq_ignore_ascii_case("jpeg")
                || ext.eq_ignore_ascii_case("png")
                || ext.eq_ignore_ascii_case("bmp")
                || ext.eq_ignore_ascii_case("webp");

            if !is_supported {
                continue;
            }

            // Canonicalize once up-front (skip entries that fail)
            let Ok(abs) = path.canonicalize() else {
                continue;
            };

            files.push(abs);
        }

        if files.is_empty() {
            anyhow::bail!("No supported images found in directory.");
        }

        log::info!("Cached {} wallpapers.", files.len());
        Ok(Self { files })
    }

    pub fn pick_random(&self) -> &Path {
        let mut rng = rand::rng();
        self.files
            .choose(&mut rng)
            .expect("WallpaperCache is non-empty after new()")
            .as_path()
    }
}
