//! On-disk PNG storage for clipboard images + thumbnails, and filesystem GC.
//!
//! Files are named by content hash, so identical images share one file (and the
//! DB dedup keeps a single row). Full image: `<hash>.png`; thumbnail:
//! `<hash>.thumb.png`.

use image::{DynamicImage, ImageBuffer};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

pub struct SavedImage {
    pub image_path: String,
    pub thumb_path: String,
    pub width: i64,
    pub height: i64,
    pub byte_size: i64,
}

/// Persist RGBA8 pixels (as returned by arboard) to a PNG + thumbnail.
pub fn save(
    images_dir: &Path,
    hash: &str,
    width: usize,
    height: usize,
    rgba: &[u8],
) -> Option<SavedImage> {
    if width == 0 || height == 0 {
        return None;
    }
    std::fs::create_dir_all(images_dir).ok()?;

    let buf: ImageBuffer<image::Rgba<u8>, Vec<u8>> =
        ImageBuffer::from_raw(width as u32, height as u32, rgba.to_vec())?;
    let dynimg = DynamicImage::ImageRgba8(buf);

    let image_path = images_dir.join(format!("{hash}.png"));
    let thumb_path = images_dir.join(format!("{hash}.thumb.png"));

    dynimg.save(&image_path).ok()?;
    // Thumbnail preserves aspect ratio within a bounding box.
    dynimg.thumbnail(360, 260).save(&thumb_path).ok()?;

    let byte_size = std::fs::metadata(&image_path)
        .map(|m| m.len() as i64)
        .unwrap_or(0);

    Some(SavedImage {
        image_path: image_path.to_string_lossy().into_owned(),
        thumb_path: thumb_path.to_string_lossy().into_owned(),
        width: width as i64,
        height: height as i64,
        byte_size,
    })
}

/// Remove the image + thumb files for a set of deleted rows.
pub fn delete_files(paths: &[(Option<String>, Option<String>)]) {
    for (full, thumb) in paths {
        if let Some(p) = full {
            let _ = std::fs::remove_file(p);
        }
        if let Some(p) = thumb {
            let _ = std::fs::remove_file(p);
        }
    }
}

/// Delete any file in `images_dir` not referenced by the DB.
pub fn gc_orphans(images_dir: &Path, referenced: &HashSet<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(images_dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() && !referenced.contains(&path) {
            let _ = std::fs::remove_file(&path);
        }
    }
}
