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
    /// Shooting details, already formatted for display ("1/250 s", "f/2.8", "35 mm (53 mm equiv.)").
    pub lens: Option<String>,
    pub exposure: Option<String>,
    pub aperture: Option<String>,
    pub iso: Option<u32>,
    pub focal_length: Option<String>,
    /// "6240 × 4160", as recorded in the EXIF data.
    pub dimensions: Option<String>,
    pub software: Option<String>,
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

/// First value of a rational tag as a number.
fn rational(x: &exif::Exif, tag: exif::Tag) -> Option<f64> {
    match &x.get_field(tag, exif::In::PRIMARY)?.value {
        exif::Value::Rational(v) => v.first().filter(|r| r.denom != 0).map(|r| r.to_f64()),
        _ => None,
    }
}

/// First value of a whole-number tag.
fn uint(x: &exif::Exif, tag: exif::Tag) -> Option<u32> {
    x.get_field(tag, exif::In::PRIMARY)?.value.get_uint(0)
}

/// A number with at most one decimal, without a trailing ".0" (2.8, 35, 0.5).
fn trim(v: f64) -> String {
    let s = format!("{:.1}", v);
    s.strip_suffix(".0").map(str::to_string).unwrap_or(s)
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
        d.lens = text(exif::Tag::LensModel).filter(|s| !s.is_empty());
        d.software = text(exif::Tag::Software).filter(|s| !s.is_empty());
        d.exposure = rational(x, exif::Tag::ExposureTime).filter(|v| *v > 0.0).map(|v| {
            if v >= 1.0 { format!("{} s", trim(v)) } else { format!("1/{} s", (1.0 / v).round()) }
        });
        d.aperture = rational(x, exif::Tag::FNumber).filter(|v| *v > 0.0).map(|v| format!("f/{}", trim(v)));
        d.iso = uint(x, exif::Tag::PhotographicSensitivity).filter(|v| *v > 0);
        d.focal_length = rational(x, exif::Tag::FocalLength).filter(|v| *v > 0.0).map(|v| {
            match uint(x, exif::Tag::FocalLengthIn35mmFilm).filter(|e| *e > 0 && f64::from(*e) != v.round()) {
                Some(eq) => format!("{} mm ({eq} mm equiv.)", trim(v)),
                None => format!("{} mm", trim(v)),
            }
        });
        let w = uint(x, exif::Tag::PixelXDimension).or_else(|| uint(x, exif::Tag::ImageWidth));
        let h = uint(x, exif::Tag::PixelYDimension).or_else(|| uint(x, exif::Tag::ImageLength));
        if let (Some(w), Some(h)) = (w, h) {
            d.dimensions = Some(format!("{w} × {h}"));
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_shooting_details() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("test-data/exif-details.jpg");
        let d = inspect(path.to_str().unwrap()).unwrap();
        assert_eq!(d.camera.as_deref(), Some("FUJIFILM X-T3"));
        assert_eq!(d.lens.as_deref(), Some("XF35mmF1.4 R"));
        assert_eq!(d.exposure.as_deref(), Some("1/250 s"));
        assert_eq!(d.aperture.as_deref(), Some("f/2.8"));
        assert_eq!(d.iso, Some(400));
        assert_eq!(d.focal_length.as_deref(), Some("35 mm (53 mm equiv.)"));
        assert_eq!(d.dimensions.as_deref(), Some("6240 × 4160"));
        assert_eq!(d.software.as_deref(), Some("darktable 4.6.1"));
        assert_eq!(d.date_taken.as_deref(), Some("2024-03-09 14:22:05"));
        assert!(d.issue.is_none());
    }

    #[test]
    fn missing_details_stay_empty() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("test-data/bad-xresolution.jpg");
        let d = inspect(path.to_str().unwrap()).unwrap();
        assert!(d.lens.is_none() && d.exposure.is_none() && d.aperture.is_none() && d.iso.is_none());
        assert_eq!(d.camera.as_deref(), Some("Canon TestCam"));
    }

    #[test]
    fn trims_numbers() {
        assert_eq!(trim(2.8), "2.8");
        assert_eq!(trim(35.0), "35");
        assert_eq!(trim(0.5), "0.5");
    }
}
