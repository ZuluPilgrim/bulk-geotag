//! Optional CSV log of errors and warnings.
//! File: <log folder>/bulk-geotag_YYYY-MM-DD_HH-MM-SS.csv (local time of the run).
//! Appends if the file already exists; writes the header only for a new file.
//! Nothing is written when there is nothing to report.

use crate::apply::{Entry, Level};
use chrono::{DateTime, Local};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};

const HEADER: &str = "time,level,folder,file,issue\n";

fn csv_field(s: &str) -> String {
    if s.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

pub fn log_file_name(started: &DateTime<Local>) -> String {
    format!("bulk-geotag_{}.csv", started.format("%Y-%m-%d_%H-%M-%S"))
}

/// Write warnings and errors from `entries`. Returns the log path if anything was written.
pub fn write_log(log_dir: &Path, started: &DateTime<Local>, entries: &[Entry]) -> Result<Option<PathBuf>, String> {
    let issues: Vec<&Entry> = entries.iter().filter(|e| e.level != Level::Info).collect();
    if issues.is_empty() {
        return Ok(None);
    }
    if !log_dir.is_dir() {
        return Err(format!("log folder not found: {}", log_dir.display()));
    }
    let path = log_dir.join(log_file_name(started));
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| format!("cannot open log {}: {e}", path.display()))?;
    let is_new = file.metadata().map(|m| m.len() == 0).unwrap_or(true);

    let mut out = String::new();
    if is_new {
        out.push_str(HEADER);
    }
    for e in issues {
        let p = Path::new(&e.path);
        let folder = p.parent().map(|f| f.display().to_string()).unwrap_or_default();
        let name = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| e.path.clone());
        out.push_str(&format!(
            "{},{},{},{},{}\n",
            csv_field(&e.time),
            e.level.as_str(),
            csv_field(&folder),
            csv_field(&name),
            csv_field(&e.message)
        ));
    }
    file.write_all(out.as_bytes())
        .map_err(|e| format!("cannot write log {}: {e}", path.display()))?;
    Ok(Some(path))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_csv() {
        assert_eq!(csv_field("plain"), "plain");
        assert_eq!(csv_field("a,b"), "\"a,b\"");
        assert_eq!(csv_field("say \"hi\""), "\"say \"\"hi\"\"\"");
    }
}
