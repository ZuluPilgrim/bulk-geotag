//! Preview (dry run) and Apply: write GPS coordinates into photos.
//!
//! For each photo that gets written:
//!   1. if backups are on, copy the original into the mirrored backup path (keep an existing backup)
//!   2. copy the photo to a hidden temp file in the same folder
//!   3. write GPS tags into the temp file
//!   4. re-read the temp file: coordinates must match and key camera tags must be unchanged
//!   5. if "update modified date" is off, restore the original modified/accessed times
//!   6. rename the temp file over the original (atomic on the same drive)
//! Any failure removes the temp file and leaves the original untouched.

use crate::scan::{self, GpsState};
use chrono::Local;
use little_exif::exif_tag::ExifTag;
use little_exif::metadata::Metadata;
use little_exif::rational::uR64;
use serde::{Deserialize, Serialize};
use std::fs::{self, File, FileTimes};
use std::io::BufReader;
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    Info,
    Warning,
    Error,
}

impl Level {
    pub fn as_str(&self) -> &'static str {
        match self {
            Level::Info => "info",
            Level::Warning => "warning",
            Level::Error => "error",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Entry {
    pub time: String,
    pub level: Level,
    pub path: String,
    pub message: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyRequest {
    pub root: String,
    pub recursive: bool,
    pub overwrite: bool,
    pub lat: f64,
    pub lon: f64,
    pub backup_dir: Option<String>,
    pub log_dir: Option<String>,
    pub dry_run: bool,
    /// true: photo's modified date becomes now, so OneDrive / sync tools notice the change.
    /// false (default): keep the original modified and accessed dates.
    #[serde(default)]
    pub update_modified: bool,
}

#[derive(Debug, Serialize)]
pub struct ApplyResult {
    pub dry_run: bool,
    pub total: usize,
    pub written: usize,
    pub skipped: usize,
    pub warnings: usize,
    pub errors: usize,
    pub entries: Vec<Entry>,
    pub log_file: Option<String>,
    pub log_error: Option<String>,
}

/// Formats we can write today. PNG is deliberately excluded for now.
fn writable(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map_or(false, |e| ["jpg", "jpeg", "tif", "tiff", "webp"].iter().any(|x| x.eq_ignore_ascii_case(e)))
}

/// Mirror the full source path under the backup folder, minus drive letter / root.
/// /home/fred/Photos/a.jpg -> <backup>/home/fred/Photos/a.jpg
/// D:\Photos\a.jpg         -> <backup>\Photos\a.jpg
pub fn backup_target(backup_dir: &Path, source: &Path) -> PathBuf {
    let mut target = backup_dir.to_path_buf();
    for c in source.components() {
        if let Component::Normal(part) = c {
            target.push(part);
        }
    }
    target
}

fn validate(req: &ApplyRequest) -> Result<(), String> {
    if !(-90.0..=90.0).contains(&req.lat) || !(-180.0..=180.0).contains(&req.lon) {
        return Err("Latitude must be −90…90 and longitude −180…180.".into());
    }
    if let Some(b) = &req.backup_dir {
        let b = Path::new(b);
        if !b.is_dir() {
            return Err(format!("Backup folder not found: {}", b.display()));
        }
        let root = fs::canonicalize(&req.root).map_err(|e| format!("Photo folder: {e}"))?;
        let backup = fs::canonicalize(b).map_err(|e| format!("Backup folder: {e}"))?;
        if backup.starts_with(&root) {
            return Err("The backup folder is inside the photo folder. Choose a backup folder outside it, \
                        otherwise backups would be scanned and tagged on the next run."
                .into());
        }
    }
    Ok(())
}

pub fn run(req: &ApplyRequest, progress: &dyn Fn(usize, usize)) -> Result<ApplyResult, String> {
    validate(req)?;
    let started = Local::now();
    let scan = scan::scan(&req.root, req.recursive)?;
    let backup_dir = req.backup_dir.as_deref().map(Path::new);

    let mut entries: Vec<Entry> = Vec::new();
    let mut add = |level: Level, path: &str, message: String| {
        entries.push(Entry { time: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(), level, path: path.to_string(), message });
    };

    for f in &scan.folder_errors {
        add(Level::Error, &f.path, f.issue.clone().unwrap_or_default());
    }

    let (mut written, mut skipped) = (0, 0);
    let total = scan.files.len();
    for (i, photo) in scan.files.iter().enumerate() {
        progress(i, total);
        let path = Path::new(&photo.path);

        if photo.gps == GpsState::Unreadable {
            add(Level::Error, &photo.path, format!("skipped: {}", photo.issue.clone().unwrap_or_default()));
            skipped += 1;
            continue;
        }
        if !writable(path) {
            add(Level::Warning, &photo.path, "skipped: PNG not supported yet".into());
            skipped += 1;
            continue;
        }
        if photo.gps == GpsState::Present && !req.overwrite {
            add(Level::Warning, &photo.path, "skipped: already has GPS".into());
            skipped += 1;
            continue;
        }

        // Backup
        if let Some(bdir) = backup_dir {
            let target = backup_target(bdir, path);
            if target.exists() {
                add(Level::Warning, &photo.path, format!("backup already exists, kept older backup: {}", target.display()));
            } else if !req.dry_run {
                let copied = target
                    .parent()
                    .map_or(Ok(()), fs::create_dir_all)
                    .and_then(|_| fs::copy(path, &target).map(|_| ()))
                    .and_then(|_| copy_file_dates(path, &target));
                if let Err(e) = copied {
                    add(Level::Error, &photo.path, format!("backup failed, photo not changed: {e}"));
                    skipped += 1;
                    continue;
                }
            }
        }

        let verb = if photo.gps == GpsState::Present { "overwrite GPS" } else { "write GPS" };
        if req.dry_run {
            add(Level::Info, &photo.path, format!("will {verb}"));
            written += 1;
            continue;
        }
        match tag_photo(path, req.lat, req.lon, !req.update_modified) {
            Ok(()) => {
                add(Level::Info, &photo.path, format!("{verb}: done"));
                written += 1;
            }
            Err(e) => {
                add(Level::Error, &photo.path, format!("not changed: {e}"));
                skipped += 1;
            }
        }
    }
    progress(total, total);

    let (mut log_file, mut log_error) = (None, None);
    if let (Some(dir), false) = (&req.log_dir, req.dry_run) {
        match crate::log::write_log(Path::new(dir), &started, &entries) {
            Ok(p) => log_file = p.map(|p| p.display().to_string()),
            Err(e) => log_error = Some(e),
        }
    }

    Ok(ApplyResult {
        dry_run: req.dry_run,
        total,
        written,
        skipped,
        warnings: entries.iter().filter(|e| e.level == Level::Warning).count(),
        errors: entries.iter().filter(|e| e.level == Level::Error).count(),
        entries,
        log_file,
        log_error,
    })
}

/// Give `to` the same modified/accessed dates as `from` (fs::copy only keeps permissions),
/// so a backup is a faithful copy of the original.
fn copy_file_dates(from: &Path, to: &Path) -> std::io::Result<()> {
    let meta = fs::metadata(from)?;
    let mut times = FileTimes::new();
    if let Ok(m) = meta.modified() {
        times = times.set_modified(m);
    }
    if let Ok(a) = meta.accessed() {
        times = times.set_accessed(a);
    }
    File::options().write(true).open(to)?.set_times(times)
}

// ---------------------------------------------------------------------------
// Writing one photo
// ---------------------------------------------------------------------------

fn to_dms(value: f64) -> Vec<uR64> {
    let v = value.abs();
    let deg = v.trunc();
    let min_f = (v - deg) * 60.0;
    let min = min_f.trunc();
    let sec = (min_f - min) * 60.0;
    vec![
        uR64 { nominator: deg as u32, denominator: 1 },
        uR64 { nominator: min as u32, denominator: 1 },
        uR64 { nominator: (sec * 10_000.0).round() as u32, denominator: 10_000 },
    ]
}

/// Read EXIF with kamadak-exif. Ok(None) = file has no EXIF at all.
fn read_exif(path: &Path) -> Result<Option<exif::Exif>, String> {
    let file = File::open(path).map_err(|e| format!("cannot open: {e}"))?;
    match exif::Reader::new().read_from_container(&mut BufReader::new(file)) {
        Ok(d) => Ok(Some(d)),
        Err(exif::Error::NotFound(_)) => Ok(None),
        Err(e) => Err(format!("cannot read metadata: {e}")),
    }
}

/// Camera tags that must survive the write unchanged.
const KEEP_TAGS: [exif::Tag; 5] = [
    exif::Tag::Make,
    exif::Tag::Model,
    exif::Tag::DateTimeOriginal,
    exif::Tag::Orientation,
    exif::Tag::DateTime,
];

fn key_tags(data: &Option<exif::Exif>) -> Vec<Option<String>> {
    KEEP_TAGS
        .iter()
        .map(|t| {
            data.as_ref()
                .and_then(|d| d.get_field(*t, exif::In::PRIMARY))
                .map(|f| f.display_value().to_string())
        })
        .collect()
}

fn read_coord(d: &exif::Exif, value: exif::Tag, reference: exif::Tag) -> Option<f64> {
    let v = match &d.get_field(value, exif::In::PRIMARY)?.value {
        exif::Value::Rational(r) if r.len() == 3 => r[0].to_f64() + r[1].to_f64() / 60.0 + r[2].to_f64() / 3600.0,
        _ => return None,
    };
    let r = d.get_field(reference, exif::In::PRIMARY)?.display_value().to_string();
    Some(if r.contains('S') || r.contains('W') { -v } else { v })
}

fn temp_path(path: &Path) -> Result<PathBuf, String> {
    let dir = path.parent().ok_or("no parent folder")?;
    let stem = path.file_stem().ok_or("no file name")?.to_string_lossy();
    let ext = path.extension().ok_or("no extension")?.to_string_lossy();
    // Keep the extension (little_exif picks the format from it); leading dot hides it from scans.
    Ok(dir.join(format!(".{stem}.bulkgeotag-tmp.{ext}")))
}

/// `keep_dates`: restore the original modified/accessed times after writing.
pub fn tag_photo(path: &Path, lat: f64, lon: f64, keep_dates: bool) -> Result<(), String> {
    let original = read_exif(path)?;
    let before = key_tags(&original);
    let meta = fs::metadata(path).map_err(|e| format!("cannot read file info: {e}"))?;
    let temp = temp_path(path)?;

    let result = (|| -> Result<(), String> {
        fs::copy(path, &temp).map_err(|e| format!("cannot create temp copy: {e}"))?;

        let mut md = if original.is_some() {
            Metadata::new_from_path(&temp).map_err(|e| format!("cannot load existing metadata safely: {e}"))?
        } else {
            Metadata::new()
        };
        md.set_tag(ExifTag::GPSVersionID(vec![2, 3, 0, 0]));
        md.set_tag(ExifTag::GPSLatitudeRef(if lat < 0.0 { "S" } else { "N" }.to_string()));
        md.set_tag(ExifTag::GPSLatitude(to_dms(lat)));
        md.set_tag(ExifTag::GPSLongitudeRef(if lon < 0.0 { "W" } else { "E" }.to_string()));
        md.set_tag(ExifTag::GPSLongitude(to_dms(lon)));
        md.write_to_file(&temp).map_err(|e| format!("cannot write GPS: {e}"))?;

        // Verify
        let after = read_exif(&temp)?.ok_or("GPS missing after write")?;
        let got_lat = read_coord(&after, exif::Tag::GPSLatitude, exif::Tag::GPSLatitudeRef).ok_or("latitude missing after write")?;
        let got_lon = read_coord(&after, exif::Tag::GPSLongitude, exif::Tag::GPSLongitudeRef).ok_or("longitude missing after write")?;
        if (got_lat - lat).abs() > 1e-5 || (got_lon - lon).abs() > 1e-5 {
            return Err(format!("verification failed: wrote {lat},{lon} but read back {got_lat},{got_lon}"));
        }
        if key_tags(&Some(after)) != before {
            return Err("verification failed: existing camera data would have changed".into());
        }

        // Keep the original timestamps, unless the user wants sync tools to see the change.
        // (Otherwise the temp file's modified date is already "now" from the write.)
        if keep_dates {
            let mut times = FileTimes::new();
            if let Ok(m) = meta.modified() {
                times = times.set_modified(m);
            }
            if let Ok(a) = meta.accessed() {
                times = times.set_accessed(a);
            }
            File::options()
                .write(true)
                .open(&temp)
                .and_then(|f| f.set_times(times))
                .map_err(|e| format!("cannot restore file dates: {e}"))?;
        }

        fs::rename(&temp, path).map_err(|e| format!("cannot replace original: {e}"))
    })();

    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backup_mirrors_full_path() {
        let t = backup_target(Path::new("/media/backup"), Path::new("/home/fred/Photos/2024/a.jpg"));
        assert_eq!(t, PathBuf::from("/media/backup/home/fred/Photos/2024/a.jpg"));
    }

    #[test]
    fn dms_round_trip() {
        let d = to_dms(-44.6414);
        let v = d[0].nominator as f64 + d[1].nominator as f64 / 60.0 + d[2].nominator as f64 / d[2].denominator as f64 / 3600.0;
        assert!((v - 44.6414).abs() < 1e-7);
    }

    #[test]
    fn backup_copy_keeps_dates() {
        let dir = std::env::temp_dir().join(format!("bg-dates-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let (a, b) = (dir.join("a.jpg"), dir.join("b.jpg"));
        fs::write(&a, b"x").unwrap();
        let old = std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_100_000_000);
        File::options().write(true).open(&a).unwrap().set_times(FileTimes::new().set_modified(old)).unwrap();
        fs::copy(&a, &b).unwrap();
        copy_file_dates(&a, &b).unwrap();
        assert_eq!(fs::metadata(&b).unwrap().modified().unwrap(), old);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn temp_keeps_extension_and_is_hidden() {
        let t = temp_path(Path::new("/p/IMG_1.JPG")).unwrap();
        assert_eq!(t, PathBuf::from("/p/.IMG_1.bulkgeotag-tmp.JPG"));
    }
}
