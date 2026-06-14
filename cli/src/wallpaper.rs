use rand::prelude::IndexedRandom;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// A collection of discovered image files available for selection.
///
/// The cache stores absolute paths to ensure that renderers (like `swww` or `swaybg`)
/// can resolve the files regardless of the current working directory.
pub struct WallpaperCache {
    /// Internal list of absolute/canonical paths to supported image files.
    files: Vec<PathBuf>,
}

impl WallpaperCache {
    /// Creates a new cache by recursively scanning the provided directory.
    ///
    /// This will traverse all subdirectories and filter for common image extensions:
    /// JPG, JPEG, PNG, BMP, and WEBP.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The directory cannot be read.
    /// - No supported image files are found.
    pub fn new<P: AsRef<Path>>(dir: P) -> anyhow::Result<Self> {
        // Ensure the theme directory/fallback CSS exists before we start picking wallpapers
        crate::theme::ensure_theme_exists()?;
        let mut files = Vec::new();

        // Recursively walk through the directory
        for entry in WalkDir::new(dir) {
            let entry = entry?;
            if !entry.file_type().is_file() {
                continue;
            }

            let path = entry.path();

            // Filter by file extension
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

            // Store canonicalized absolute paths to prevent resolution issues later.
            // We skip files that cannot be canonicalized (e.g., permission issues).
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

    /// Selects a random wallpaper from the cache.
    ///
    /// # Panics
    ///
    /// Panics if the cache is empty, though the `new` constructor
    /// guarantees at least one file is present.
    pub fn pick_random(&self) -> &Path {
        let mut rng = rand::rng();
        self.files
            .choose(&mut rng)
            .expect("WallpaperCache is non-empty after new()")
            .as_path()
    }
}
