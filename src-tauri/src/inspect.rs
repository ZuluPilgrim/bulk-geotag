//! Single-file mode: read what a photo already has (GPS, date taken, camera) and a preview image.
//! Read-only — never modifies the file.

use crate::apply::{date_text, read_coord, read_exif};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::Serialize;
use std::fs;
use std::path::Path;

/// Don't send very large photos to the UI as a preview.
const MAX_PREVIEW_BYTES: u64 = 25 * 1024 * 1024;

#[derive(Debug, Serialize, Default)]
pub struct PhotoDetails {
    pub path: String,
    pub name: String,
    /// Current GPS, if any.
    pub lat: Option<f64>,
    pub lon: Option<f64>,
    /// Current Date Taken as "YYYY-MM-DD HH:MM:SS", if any.
    pub date_taken: Option<String>,
    pub camera: Option<String>,
    /// EXIF orientation 1–8 (applies to an embedded thumbnail; full images are rotated by the viewer).
    pub orientation: u32,
    /// `data:` URL for the preview, or None (e.g. TIFF, too large).
    pub preview: Option<String>,
    /// True when `preview` is the small thumbnail embedded in the EXIF data.
    pub preview_is_thumbnail: bool,
    /// Problem reading the metadata, if any.
    pub issue: Option<String>,
}

fn mime_for(path: &Path) -> Option<&'static str> {
    match path.extension()?.to_str()?.to_ascii_lowercase().as_str() {
        "jpg" | "jpeg" => Some("image/jpeg"),
        "webp" => Some("image/webp"),
        "png" => Some("image/png"),
        _ => None, // TIFF: the window's web view can't display it
    }
}

/// The thumbnail JPEG embedded in the EXIF data (IFD1), if present.
fn embedded_thumbnail(data: &exif::Exif) -> Option<Vec<u8>> {
    let num = |tag| data.get_field(tag, exif::In::THUMBNAIL).and_then(|f| f.value.get_uint(0)).map(|v| v as usize);
    let offset = num(exif::Tag::JPEGInterchangeFormat)?;
    let length = num(exif::Tag::JPEGInterchangeFormatLength)?;
    data.buf().get(offset..offset.checked_add(length)?).map(<[u8]>::to_vec)
}

pub fn inspect(path_str: &str) -> Result<PhotoDetails, String> {
    let path = Path::new(path_str);
    if !path.is_file() {
        return Err(format!("Not a file: {path_str}"));
    }
    if !crate::scan::is_supported(path) {
        return Err(format!("Not a supported photo (jpg, jpeg, png, tif, tiff, webp): {path_str}"));
    }
    let mut d = PhotoDetails {
        path: path.display().to_string(),
        name: path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(),
        orientation: 1,
        ..Default::default()
    };

    let exif_data = match read_exif(path) {
        Ok(x) => x,
        Err(e) => {
            d.issue = Some(e);
            None
        }
    };
    if let Some(x) = &exif_data {
        d.lat = read_coord(x, exif::Tag::GPSLatitude, exif::Tag::GPSLatitudeRef);
        d.lon = read_coord(x, exif::Tag::GPSLongitude, exif::Tag::GPSLongitudeRef);
        if d.lat.is_none() || d.lon.is_none() {
            (d.lat, d.lon) = (None, None);
        }
        // "YYYY:MM:DD HH:MM:SS" -> "YYYY-MM-DD HH:MM:SS"
        d.date_taken = date_text(x, exif::Tag::DateTimeOriginal).map(|s| s.replacen(':', "-", 2));
        let text = |tag| x.get_field(tag, exif::In::PRIMARY).map(|f| f.display_value().to_string().trim_matches('"').trim().to_string());
        d.camera = match (text(exif::Tag::Make), text(exif::Tag::Model)) {
            (Some(make), Some(model)) if model.starts_with(&make) => Some(model),
            (Some(make), Some(model)) => Some(format!("{make} {model}")),
            (a, b) => a.or(b),
        };
        d.orientation = x
            .get_field(exif::Tag::Orientation, exif::In::PRIMARY)
            .and_then(|f| f.value.get_uint(0))
            .unwrap_or(1);
    }

    // Preview: prefer the full image (the viewer rotates it correctly); fall back to the
    // embedded thumbnail for big files or formats the viewer can't show (e.g. TIFF).
    let size = fs::metadata(path).map(|m| m.len()).unwrap_or(u64::MAX);
    if let (Some(mime), true) = (mime_for(path), size <= MAX_PREVIEW_BYTES) {
        if let Ok(bytes) = fs::read(path) {
            d.preview = Some(format!("data:{mime};base64,{}", STANDARD.encode(bytes)));
        }
    }
    if d.preview.is_none() {
        if let Some(thumb) = exif_data.as_ref().and_then(embedded_thumbnail) {
            d.preview = Some(format!("data:image/jpeg;base64,{}", STANDARD.encode(thumb)));
            d.preview_is_thumbnail = true;
        }
    }
    Ok(d)
}
