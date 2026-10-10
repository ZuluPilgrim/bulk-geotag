//! User settings stored as JSON in the app's config folder:
//!   Linux:   ~/.config/com.zulupilgrim.bulkgeotag/settings.json
//!   Windows: %APPDATA%\com.zulupilgrim.bulkgeotag\settings.json
//!   macOS:   ~/Library/Application Support/com.zulupilgrim.bulkgeotag/settings.json
//! Created with defaults on first use. Re-read on every search, so edits apply without a restart.
//! Edited from the ⚙ Settings dialog (or by hand); missing fields fall back to their defaults.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

pub const DEFAULT_NOMINATIM_URL: &str = "https://nominatim.openstreetmap.org";
pub const DEFAULT_LANGUAGE: &str = "en";
pub const DEFAULT_MAX_RESULTS: u8 = 5;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Settings {
    /// Base URL of the Nominatim server (no trailing /search).
    pub nominatim_url: String,
    /// Optional contact address sent to Nominatim, as its usage policy recommends.
    pub contact_email: String,
    /// Language(s) for place names, as codes for Nominatim's accept-language: "en", "af", "en,mi".
    pub language: String,
    /// Results per search, 1–10.
    pub max_results: u8,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            nominatim_url: DEFAULT_NOMINATIM_URL.to_string(),
            contact_email: String::new(),
            language: DEFAULT_LANGUAGE.to_string(),
            max_results: DEFAULT_MAX_RESULTS,
        }
    }
}

impl Settings {
    /// Tidy and check values before saving. Returns the cleaned settings or a readable error.
    pub fn validated(mut self) -> Result<Self, String> {
        self.nominatim_url = self.nominatim_url.trim().trim_end_matches('/').to_string();
        if self.nominatim_url.is_empty() {
            self.nominatim_url = DEFAULT_NOMINATIM_URL.to_string();
        }
        if !(self.nominatim_url.starts_with("https://") || self.nominatim_url.starts_with("http://")) {
            return Err("Nominatim server must start with https:// (or http://).".into());
        }
        self.contact_email = self.contact_email.trim().to_string();
        if self.contact_email.chars().any(|c| c.is_control() || !c.is_ascii()) {
            return Err("Contact email can only contain plain ASCII characters.".into());
        }
        self.language = normalise_language(&self.language)?;
        if !(1..=10).contains(&self.max_results) {
            return Err("Results per search must be between 1 and 10.".into());
        }
        Ok(self)
    }
}

/// "EN, af " -> "en,af". Each code: 2–3 letters, optionally a region/script like "en-NZ" or "zh-Hans".
/// Blank -> "en".
pub fn normalise_language(input: &str) -> Result<String, String> {
    let codes: Vec<String> = input
        .split(',')
        .map(str::trim)
        .filter(|c| !c.is_empty())
        .map(|c| {
            let mut parts = c.split('-');
            let lang = parts.next().unwrap_or_default();
            let ok_lang = (2..=3).contains(&lang.len()) && lang.chars().all(|ch| ch.is_ascii_alphabetic());
            let ok_rest = parts.all(|p| (2..=8).contains(&p.len()) && p.chars().all(|ch| ch.is_ascii_alphanumeric()));
            if ok_lang && ok_rest {
                Ok(c.to_ascii_lowercase())
            } else {
                Err(format!("\"{c}\" is not a language code (e.g. en, af, mi, en-NZ)."))
            }
        })
        .collect::<Result<_, _>>()?;
    Ok(if codes.is_empty() { DEFAULT_LANGUAGE.to_string() } else { codes.join(",") })
}

pub fn settings_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|e| format!("cannot locate config folder: {e}"))?;
    Ok(dir.join("settings.json"))
}

fn write(path: &PathBuf, settings: &Settings) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
    }
    let json = serde_json::to_string_pretty(settings).expect("settings serialise");
    fs::write(path, json + "\n").map_err(|e| format!("cannot write {}: {e}", path.display()))
}

/// Load settings, creating the file with defaults if it does not exist yet.
pub fn load(app: &AppHandle) -> Result<Settings, String> {
    let path = settings_path(app)?;
    if !path.exists() {
        let defaults = Settings::default();
        write(&path, &defaults)?;
        return Ok(defaults);
    }
    let text = fs::read_to_string(&path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    serde_json::from_str(&text).map_err(|e| format!("{} is not valid JSON: {e}", path.display()))
}

/// Validate and save settings from the Settings dialog. Returns what was actually saved.
pub fn save(app: &AppHandle, settings: Settings) -> Result<Settings, String> {
    let clean = settings.validated()?;
    write(&settings_path(app)?, &clean)?;
    Ok(clean)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn language_codes() {
        assert_eq!(normalise_language(" EN, af ").unwrap(), "en,af");
        assert_eq!(normalise_language("en-NZ").unwrap(), "en-nz");
        assert_eq!(normalise_language("").unwrap(), "en");
        assert!(normalise_language("english").is_err());
        assert!(normalise_language("e1").is_err());
    }

    #[test]
    fn validation() {
        let ok = Settings { nominatim_url: " https://geo.example.com/ ".into(), ..Default::default() }.validated().unwrap();
        assert_eq!(ok.nominatim_url, "https://geo.example.com");
        assert!(Settings { nominatim_url: "ftp://x".into(), ..Default::default() }.validated().is_err());
        assert!(Settings { max_results: 0, ..Default::default() }.validated().is_err());
        assert!(Settings { max_results: 11, ..Default::default() }.validated().is_err());
        assert_eq!(Settings { nominatim_url: "".into(), ..Default::default() }.validated().unwrap().nominatim_url, DEFAULT_NOMINATIM_URL);
    }

    #[test]
    fn old_files_get_new_defaults() {
        // A v1.1 settings.json has no language / max_results.
        let s: Settings = serde_json::from_str(r#"{"nominatim_url":"https://x.org","contact_email":"a@b.c"}"#).unwrap();
        assert_eq!((s.language.as_str(), s.max_results), ("en", 5));
        assert_eq!(s.contact_email, "a@b.c");
    }
}
