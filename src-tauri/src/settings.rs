//! User settings stored as JSON in the app's config folder:
//!   Linux:   ~/.config/com.zulupilgrim.bulkgeotag/settings.json
//!   Windows: %APPDATA%\com.zulupilgrim.bulkgeotag\settings.json
//!   macOS:   ~/Library/Application Support/com.zulupilgrim.bulkgeotag/settings.json
//! Created with defaults on first use. Re-read on every search, so edits apply without a restart.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

pub const DEFAULT_NOMINATIM_URL: &str = "https://nominatim.openstreetmap.org";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Base URL of the Nominatim server (no trailing /search).
    pub nominatim_url: String,
    /// Optional contact address sent to Nominatim, as its usage policy recommends.
    pub contact_email: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            nominatim_url: DEFAULT_NOMINATIM_URL.to_string(),
            contact_email: String::new(),
        }
    }
}

pub fn settings_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|e| format!("cannot locate config folder: {e}"))?;
    Ok(dir.join("settings.json"))
}

/// Load settings, creating the file with defaults if it does not exist yet.
pub fn load(app: &AppHandle) -> Result<Settings, String> {
    let path = settings_path(app)?;
    if !path.exists() {
        let defaults = Settings::default();
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir).map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
        }
        let json = serde_json::to_string_pretty(&defaults).expect("settings serialise");
        fs::write(&path, json + "\n").map_err(|e| format!("cannot write {}: {e}", path.display()))?;
        return Ok(defaults);
    }
    let text = fs::read_to_string(&path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    serde_json::from_str(&text).map_err(|e| format!("{} is not valid JSON: {e}", path.display()))
}
