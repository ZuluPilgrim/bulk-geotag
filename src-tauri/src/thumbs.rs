//! Thumbnails for the Inspector's gallery. Read-only for photos: nothing is ever written
//! next to them. Small tiles use the JPEG embedded in the EXIF data when there is one
//! (instant); otherwise, and for large tiles, a JPEG is made with the `image` crate and kept
//! in the app's cache folder.

use crate::apply::read_exif;
use crate::inspect::embedded_thumbnail;
use base64::{engine::general_purpose::STANDARD, Engine};
use image::codecs::jpeg::JpegEncoder;
use image::ImageReader;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

/// Longest side of a generated thumbnail, in pixels: for small tiles, and for large ones.
const SMALL: u32 = 256;
const LARGE: u32 = 768;
const JPEG_QUALITY: u8 = 80;

#[derive(Debug, Serialize)]
pub struct Thumb {
    /// `data:image/jpeg;base64,…`
    pub data: String,
    /// EXIF orientation 1–8; the UI turns the picture to match (same as single-file previews).
    pub orientation: u32,
    /// True when it came from the EXIF data, false when generated (or from the cache).
    pub embedded: bool,
}

/// Thumbnail for one photo, or None if the photo can't be read as an image.
/// `large`: for big tiles; skips the embedded thumbnail (usually only ~160 px) and makes a 768 px one.
pub fn thumbnail(path: &Path, cache_dir: &Path, large: bool) -> Result<Option<Thumb>, String> {
    if !path.is_file() {
        return Err(format!("Not a file: {}", path.display()));
    }
    let exif_data = read_exif(path).ok().flatten();
    let orientation = exif_data
        .as_ref()
        .and_then(|x| x.get_field(exif::Tag::Orientation, exif::In::PRIMARY))
        .and_then(|f| f.value.get_uint(0))
        .filter(|o| (1..=8).contains(o))
        .unwrap_or(1);

    if let Some(bytes) = exif_data.as_ref().and_then(embedded_thumbnail).filter(|_| !large) {
        return Ok(Some(Thumb { data: data_url(&bytes), orientation, embedded: true }));
    }

    let size = if large { LARGE } else { SMALL };
    let cached = cache_dir.join(cache_name(path, size)?);
    if let Ok(bytes) = fs::read(&cached) {
        return Ok(Some(Thumb { data: data_url(&bytes), orientation, embedded: false }));
    }
    let Some(bytes) = generate(path, size) else { return Ok(None) };
    // Best effort: a cache that can't be written just means generating again next time.
    if fs::create_dir_all(cache_dir).is_ok() {
        let tmp = cached.with_extension("tmp");
        if fs::write(&tmp, &bytes).is_ok() {
            let _ = fs::rename(&tmp, &cached);
        }
    }
    Ok(Some(Thumb { data: data_url(&bytes), orientation, embedded: false }))
}

/// Number of cached (generated) thumbnails.
pub fn cache_count(cache_dir: &Path) -> usize {
    cached_files(cache_dir).len()
}

/// Delete every cached thumbnail. Returns how many were removed.
pub fn clear_cache(cache_dir: &Path) -> usize {
    cached_files(cache_dir).into_iter().filter(|p| fs::remove_file(p).is_ok()).count()
}

fn cached_files(cache_dir: &Path) -> Vec<PathBuf> {
    fs::read_dir(cache_dir)
        .map(|rd| {
            rd.filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| p.extension().is_some_and(|x| x == "jpg" || x == "tmp"))
                .collect()
        })
        .unwrap_or_default()
}

/// Decode, shrink and re-encode as JPEG. None if the file can't be decoded.
fn generate(path: &Path, size: u32) -> Option<Vec<u8>> {
    let img = ImageReader::open(path).ok()?.with_guessed_format().ok()?.decode().ok()?;
    let small = img.thumbnail(size, size).to_rgb8();
    let mut out = Vec::new();
    JpegEncoder::new_with_quality(&mut out, JPEG_QUALITY).encode_image(&small).ok()?;
    Some(out)
}

/// File name in the cache: a hash of the full path, file size and modified time (so an edited
/// photo gets a new thumbnail and an old one is never shown), plus the thumbnail size.
fn cache_name(path: &Path, thumb_size: u32) -> Result<String, String> {
    let meta = fs::metadata(path).map_err(|e| format!("cannot read file info: {e}"))?;
    let modified = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let key = format!("{}|{}|{}", path.display(), meta.len(), modified);
    Ok(format!("{:016x}-{thumb_size}.jpg", fnv1a(key.as_bytes())))
}

/// FNV-1a 64-bit: small, and the same on every platform and Rust version.
fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, b| (h ^ u64::from(*b)).wrapping_mul(0x0100_0000_01b3))
}

fn data_url(jpeg: &[u8]) -> String {
    format!("data:image/jpeg;base64,{}", STANDARD.encode(jpeg))
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageFormat, Rgb, RgbImage};

    fn temp_dir(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("bgt-thumbs-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    fn decoded_size(t: &Thumb) -> (u32, u32) {
        let b64 = t.data.strip_prefix("data:image/jpeg;base64,").unwrap();
        let img = image::load_from_memory(&STANDARD.decode(b64).unwrap()).unwrap();
        (img.width(), img.height())
    }

    #[test]
    fn generates_and_caches_when_no_embedded_thumbnail() {
        let dir = temp_dir("gen");
        let photo = dir.join("wide.png");
        RgbImage::from_pixel(800, 400, Rgb([200, 120, 40])).save_with_format(&photo, ImageFormat::Png).unwrap();
        let cache = dir.join("thumbs");

        let t = thumbnail(&photo, &cache, false).unwrap().unwrap();
        assert!(!t.embedded);
        assert_eq!(t.orientation, 1);
        assert_eq!(decoded_size(&t), (256, 128));
        assert_eq!(cache_count(&cache), 1);

        // Second call is served from the cache: same picture.
        let again = thumbnail(&photo, &cache, false).unwrap().unwrap();
        assert_eq!(again.data, t.data);

        // Large tiles get their own, bigger copy.
        let big = thumbnail(&photo, &cache, true).unwrap().unwrap();
        assert_eq!(decoded_size(&big), (768, 384));
        assert_eq!(cache_count(&cache), 2);

        assert_eq!(clear_cache(&cache), 2);
        assert_eq!(cache_count(&cache), 0);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn changed_photo_gets_a_new_cache_name() {
        let dir = temp_dir("key");
        let photo = dir.join("a.png");
        RgbImage::new(10, 10).save_with_format(&photo, ImageFormat::Png).unwrap();
        let first = cache_name(&photo, 256).unwrap();
        assert_ne!(cache_name(&photo, 768).unwrap(), first);
        RgbImage::new(20, 10).save_with_format(&photo, ImageFormat::Png).unwrap();
        assert_ne!(cache_name(&photo, 256).unwrap(), first);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn unreadable_image_gives_none() {
        let dir = temp_dir("bad");
        let photo = dir.join("broken.jpg");
        fs::write(&photo, b"not really a jpeg").unwrap();
        assert!(thumbnail(&photo, &dir.join("thumbs"), false).unwrap().is_none());
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn missing_cache_folder_counts_zero() {
        assert_eq!(cache_count(Path::new("/nonexistent/bgt-thumbs")), 0);
    }
}
