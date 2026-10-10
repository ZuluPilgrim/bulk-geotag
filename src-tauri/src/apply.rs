//! Preview (dry run) and Apply: write GPS coordinates and/or Date Taken into photos.
//!
//! For each photo that gets written:
//!   1. if backups are on, copy the original into the mirrored backup path (keep an existing backup)
//!   2. copy the photo to a hidden temp file in the same folder
//!   3. write the requested tags into the temp file:
//!        GPS (if a location is set), DateTimeOriginal + CreateDate (if a date is set),
//!        and always ModifyDate = now, because the EXIF data has changed
//!   4. re-read the temp file: written values must match and key camera tags must be unchanged
//!   5. if "update modified date" is off, restore the original file modified/accessed times
//!   6. rename the temp file over the original (atomic on the same drive)
//! Any failure removes the temp file and leaves the original untouched.

use crate::scan::{self, GpsState};
use chrono::{Local, NaiveDateTime};
use little_exif::exif_tag::ExifTag;
use little_exif::metadata::Metadata;
use little_exif::rational::uR64;
use serde::{Deserialize, Serialize};
use std::fs::{self, File, FileTimes};
use std::io::BufReader;
use std::path::{Component, Path, PathBuf};

/// EXIF date format: "YYYY:MM:DD HH:MM:SS" (local time, no time zone).
const EXIF_DATE: &str = "%Y:%m:%d %H:%M:%S";

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
    /// Replace GPS that is already there.
    pub overwrite: bool,
    /// Location to write. Both None = don't change GPS.
    #[serde(default)]
    pub lat: Option<f64>,
    #[serde(default)]
    pub lon: Option<f64>,
    /// Date Taken to write, from the UI as "YYYY-MM-DDTHH:MM[:SS]". None = don't change it.
    #[serde(default)]
    pub date_taken: Option<String>,
    /// Replace a Date Taken that is already there.
    #[serde(default)]
    pub overwrite_date: bool,
    pub backup_dir: Option<String>,
    pub log_dir: Option<String>,
    pub dry_run: bool,
    /// true: photo file's modified date becomes now, so OneDrive / sync tools notice the change.
    /// false (default): keep the original file modified and accessed dates.
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

/// What to write into one photo.
#[derive(Debug, Clone, Copy)]
pub struct Changes<'a> {
    pub gps: Option<(f64, f64)>,
    /// Already in EXIF format.
    pub date: Option<&'a str>,
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

/// "2024-05-01T10:30" or "2024-05-01T10:30:15" (HTML datetime-local) -> "2024:05:01 10:30:00".
pub fn to_exif_date(input: &str) -> Result<String, String> {
    let s = input.trim();
    NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S")
        .or_else(|_| NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M"))
        .map(|d| d.format(EXIF_DATE).to_string())
        .map_err(|_| format!("Date/time not understood: {s}"))
}

fn validate(req: &ApplyRequest) -> Result<(Option<(f64, f64)>, Option<String>), String> {
    let gps = match (req.lat, req.lon) {
        (Some(lat), Some(lon)) => {
            if !(-90.0..=90.0).contains(&lat) || !(-180.0..=180.0).contains(&lon) {
                return Err("Latitude must be −90…90 and longitude −180…180.".into());
            }
            Some((lat, lon))
        }
        (None, None) => None,
        _ => return Err("Both latitude and longitude are needed.".into()),
    };
    let date = match req.date_taken.as_deref().map(str::trim) {
        Some(d) if !d.is_empty() => Some(to_exif_date(d)?),
        _ => None,
    };
    if gps.is_none() && date.is_none() {
        return Err("Nothing to write: set a location, a date taken, or both.".into());
    }
    if let Some(b) = &req.backup_dir {
        let b = Path::new(b);
        if !b.is_dir() {
            return Err(format!("Backup folder not found: {}", b.display()));
        }
        let mut root = fs::canonicalize(&req.root).map_err(|e| format!("Photo folder: {e}"))?;
        if root.is_file() {
            // Single file: the rule applies to the folder it lives in.
            root = root.parent().map(Path::to_path_buf).unwrap_or(root);
        }
        let backup = fs::canonicalize(b).map_err(|e| format!("Backup folder: {e}"))?;
        if backup.starts_with(&root) {
            return Err("The backup folder is inside the photo's folder. Choose a backup folder outside it, \
                        otherwise backups would be scanned and tagged on the next run."
                .into());
        }
    }
    Ok((gps, date))
}

pub fn run(req: &ApplyRequest, progress: &dyn Fn(usize, usize)) -> Result<ApplyResult, String> {
    let (gps, date) = validate(req)?;
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

        // Decide what this photo gets.
        let mut changes = Changes { gps: None, date: None };
        if gps.is_some() {
            if photo.gps == GpsState::Present && !req.overwrite {
                add(Level::Warning, &photo.path, "GPS not changed: already has GPS".into());
            } else {
                changes.gps = gps;
            }
        }
        if let Some(d) = date.as_deref() {
            if photo.has_date && !req.overwrite_date {
                add(Level::Warning, &photo.path, "date taken not changed: already has one".into());
            } else {
                changes.date = Some(d);
            }
        }
        if changes.gps.is_none() && changes.date.is_none() {
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

        let mut what = Vec::new();
        if changes.gps.is_some() {
            what.push(if photo.gps == GpsState::Present { "overwrite GPS" } else { "write GPS" });
        }
        if changes.date.is_some() {
            what.push(if photo.has_date { "overwrite date taken" } else { "write date taken" });
        }
        let what = what.join(" + ");

        if req.dry_run {
            add(Level::Info, &photo.path, format!("will {what}"));
            written += 1;
            continue;
        }
        match tag_photo(path, changes, !req.update_modified) {
            Ok(()) => {
                add(Level::Info, &photo.path, format!("{what}: done"));
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
pub(crate) fn read_exif(path: &Path) -> Result<Option<exif::Exif>, String> {
    let file = File::open(path).map_err(|e| format!("cannot open: {e}"))?;
    match exif::Reader::new().read_from_container(&mut BufReader::new(file)) {
        Ok(d) => Ok(Some(d)),
        Err(exif::Error::NotFound(_)) => Ok(None),
        Err(e) => Err(format!("cannot read metadata: {e}")),
    }
}

fn tag_text(data: Option<&exif::Exif>, tag: exif::Tag) -> Option<String> {
    data.and_then(|d| d.get_field(tag, exif::In::PRIMARY))
        .map(|f| f.display_value().to_string())
}

/// EXIF date tag as "YYYY:MM:DD HH:MM:SS" (raw text, so it compares exactly).
pub(crate) fn date_text(data: &exif::Exif, tag: exif::Tag) -> Option<String> {
    match &data.get_field(tag, exif::In::PRIMARY)?.value {
        exif::Value::Ascii(v) => v.first().map(|b| String::from_utf8_lossy(b).trim_end_matches('\0').trim().to_string()),
        _ => None,
    }
}

/// Camera tags that must survive every write unchanged.
const KEEP_TAGS: [exif::Tag; 3] = [exif::Tag::Make, exif::Tag::Model, exif::Tag::Orientation];

pub(crate) fn read_coord(d: &exif::Exif, value: exif::Tag, reference: exif::Tag) -> Option<f64> {
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

/// Write `changes` into one photo. `keep_dates`: restore the file's original modified/accessed times.
pub fn tag_photo(path: &Path, changes: Changes, keep_dates: bool) -> Result<(), String> {
    let original = read_exif(path)?;
    let keep_before: Vec<Option<String>> = KEEP_TAGS.iter().map(|t| tag_text(original.as_ref(), *t)).collect();
    let taken_before = original.as_ref().and_then(|d| date_text(d, exif::Tag::DateTimeOriginal));
    let meta = fs::metadata(path).map_err(|e| format!("cannot read file info: {e}"))?;
    let temp = temp_path(path)?;
    let now = Local::now().format(EXIF_DATE).to_string();

    let result = (|| -> Result<(), String> {
        fs::copy(path, &temp).map_err(|e| format!("cannot create temp copy: {e}"))?;

        let mut md = if original.is_some() {
            Metadata::new_from_path(&temp).map_err(|e| format!("cannot load existing metadata safely: {e}"))?
        } else {
            Metadata::new()
        };
        if let Some((lat, lon)) = changes.gps {
            md.set_tag(ExifTag::GPSVersionID(vec![2, 3, 0, 0]));
            md.set_tag(ExifTag::GPSLatitudeRef(if lat < 0.0 { "S" } else { "N" }.to_string()));
            md.set_tag(ExifTag::GPSLatitude(to_dms(lat)));
            md.set_tag(ExifTag::GPSLongitudeRef(if lon < 0.0 { "W" } else { "E" }.to_string()));
            md.set_tag(ExifTag::GPSLongitude(to_dms(lon)));
        }
        if let Some(d) = changes.date {
            md.set_tag(ExifTag::DateTimeOriginal(d.to_string()));
            md.set_tag(ExifTag::CreateDate(d.to_string()));
        }
        // The EXIF data changed, so its own "modified" stamp is set to now.
        md.set_tag(ExifTag::ModifyDate(now.clone()));
        md.write_to_file(&temp).map_err(|e| format!("cannot write EXIF: {e}"))?;

        // Verify
        let after = read_exif(&temp)?.ok_or("EXIF missing after write")?;
        if let Some((lat, lon)) = changes.gps {
            let got_lat = read_coord(&after, exif::Tag::GPSLatitude, exif::Tag::GPSLatitudeRef).ok_or("latitude missing after write")?;
            let got_lon = read_coord(&after, exif::Tag::GPSLongitude, exif::Tag::GPSLongitudeRef).ok_or("longitude missing after write")?;
            if (got_lat - lat).abs() > 1e-5 || (got_lon - lon).abs() > 1e-5 {
                return Err(format!("verification failed: wrote {lat},{lon} but read back {got_lat},{got_lon}"));
            }
        }
        let taken_after = date_text(&after, exif::Tag::DateTimeOriginal);
        match changes.date {
            Some(d) => {
                let created = date_text(&after, exif::Tag::DateTimeDigitized);
                if taken_after.as_deref() != Some(d) || created.as_deref() != Some(d) {
                    return Err(format!("verification failed: date taken reads back as {taken_after:?}, expected {d}"));
                }
            }
            None => {
                if taken_after != taken_before {
                    return Err("verification failed: date taken would have changed".into());
                }
            }
        }
        if date_text(&after, exif::Tag::DateTime).as_deref() != Some(now.as_str()) {
            return Err("verification failed: EXIF modified date not updated".into());
        }
        let keep_after: Vec<Option<String>> = KEEP_TAGS.iter().map(|t| tag_text(Some(&after), *t)).collect();
        if keep_after != keep_before {
            return Err("verification failed: existing camera data would have changed".into());
        }

        // Keep the file's original timestamps, unless the user wants sync tools to see the change.
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

    #[test]
    fn exif_date_from_ui_input() {
        assert_eq!(to_exif_date("2024-05-01T10:30").unwrap(), "2024:05:01 10:30:00");
        assert_eq!(to_exif_date("2024-05-01T10:30:15").unwrap(), "2024:05:01 10:30:15");
        assert!(to_exif_date("yesterday").is_err());
    }
}
