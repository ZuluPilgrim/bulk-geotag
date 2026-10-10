//! Folder scan: finds supported photos and reports which already have GPS.
//! Read-only — never modifies any file.

use serde::Serialize;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use walkdir::{DirEntry, WalkDir};

/// File extensions we can read and (later) write GPS for. Compared case-insensitively.
const EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "tif", "tiff", "webp"];

#[derive(Debug, Serialize, PartialEq, Eq, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum GpsState {
    /// Has GPS latitude and longitude tags.
    Present,
    /// Readable, but no GPS (or no EXIF at all).
    Absent,
    /// Could not be read — corrupt metadata, not really an image, or an I/O error.
    Unreadable,
}

#[derive(Debug, Serialize)]
pub struct PhotoInfo {
    /// Full path to the file.
    pub path: String,
    pub gps: GpsState,
    /// Has an EXIF Date Taken (DateTimeOriginal).
    pub has_date: bool,
    /// Reason, when `gps` is `Unreadable`.
    pub issue: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ScanResult {
    pub photos: usize,
    pub with_gps: usize,
    pub unreadable: usize,
    pub files: Vec<PhotoInfo>,
    /// Folders that could not be opened (e.g. permission denied). Not counted as photos.
    pub folder_errors: Vec<PhotoInfo>,
}

fn is_hidden(entry: &DirEntry) -> bool {
    // Never treat the folder the user picked as hidden, even if its name starts with "."
    entry.depth() > 0 && entry.file_name().to_str().map_or(false, |n| n.starts_with('.'))
}

pub fn is_supported(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map_or(false, |e| EXTENSIONS.iter().any(|x| x.eq_ignore_ascii_case(e)))
}

/// Inspect one file's EXIF: GPS state, whether it has a Date Taken, and any problem.
pub fn read_gps_state(path: &Path) -> (GpsState, bool, Option<String>) {
    let file = match File::open(path) {
        Ok(f) => f,
        Err(e) => return (GpsState::Unreadable, false, Some(format!("cannot open: {e}"))),
    };
    let mut reader = BufReader::new(file);
    match exif::Reader::new().read_from_container(&mut reader) {
        Ok(data) => {
            let has_lat = data.get_field(exif::Tag::GPSLatitude, exif::In::PRIMARY).is_some();
            let has_lon = data.get_field(exif::Tag::GPSLongitude, exif::In::PRIMARY).is_some();
            let has_date = data.get_field(exif::Tag::DateTimeOriginal, exif::In::PRIMARY).is_some();
            let gps = if has_lat && has_lon { GpsState::Present } else { GpsState::Absent };
            (gps, has_date, None)
        }
        // A valid image that simply carries no EXIF block is normal, not an error.
        Err(exif::Error::NotFound(_)) => (GpsState::Absent, false, None),
        Err(e) => (GpsState::Unreadable, false, Some(format!("unreadable metadata: {e}"))),
    }
}

/// Scan `root` for supported photos. `root` may be a folder or a single photo file.
/// Hidden files/folders are skipped and symbolic links are not followed.
pub fn scan(root: &str, recursive: bool) -> Result<ScanResult, String> {
    let root_path = Path::new(root);
    // Single-file mode: the "root" is one photo.
    if root_path.is_file() {
        if !is_supported(root_path) {
            return Err(format!("Not a supported photo (jpg, jpeg, png, tif, tiff, webp): {root}"));
        }
        let (gps, has_date, issue) = read_gps_state(root_path);
        let files = vec![PhotoInfo { path: root_path.display().to_string(), gps, has_date, issue }];
        return Ok(ScanResult {
            photos: 1,
            with_gps: (gps == GpsState::Present) as usize,
            unreadable: (gps == GpsState::Unreadable) as usize,
            files,
            folder_errors: Vec::new(),
        });
    }
    if !root_path.is_dir() {
        return Err(format!("Not a folder or file: {root}"));
    }

    let walker = WalkDir::new(root_path)
        .follow_links(false)
        .max_depth(if recursive { usize::MAX } else { 1 })
        .sort_by_file_name()
        .into_iter()
        .filter_entry(|e| !is_hidden(e));

    let mut files = Vec::new();
    let mut folder_errors = Vec::new();
    for entry in walker {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                // Unreadable subfolder (e.g. permission denied): record it and carry on.
                let path = e.path().map(|p| p.display().to_string()).unwrap_or_else(|| root.to_string());
                folder_errors.push(PhotoInfo { path, gps: GpsState::Unreadable, has_date: false, issue: Some(format!("cannot read: {e}")) });
                continue;
            }
        };
        if !entry.file_type().is_file() || !is_supported(entry.path()) {
            continue;
        }
        let (gps, has_date, issue) = read_gps_state(entry.path());
        files.push(PhotoInfo { path: entry.path().display().to_string(), gps, has_date, issue });
    }

    Ok(ScanResult {
        photos: files.len(),
        with_gps: files.iter().filter(|f| f.gps == GpsState::Present).count(),
        unreadable: files.iter().filter(|f| f.gps == GpsState::Unreadable).count(),
        files,
        folder_errors,
    })
}
