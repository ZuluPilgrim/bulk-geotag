mod apply;
mod exif_repair;
mod geocode;
mod inspect;
mod log;
mod scan;
mod settings;

use tauri::{Emitter, Manager};

/// Scan a folder for supported photos. Runs on a background thread so the UI stays responsive.
#[tauri::command]
async fn scan_folder(path: String, recursive: bool) -> Result<scan::ScanResult, String> {
    tauri::async_runtime::spawn_blocking(move || scan::scan(&path, recursive))
        .await
        .map_err(|e| format!("scan task failed: {e}"))?
}

/// Look up a place name. Settings are re-read each time so edits to settings.json apply immediately.
#[tauri::command]
async fn geocode(
    app: tauri::AppHandle,
    geocoder: tauri::State<'_, geocode::Geocoder>,
    query: String,
) -> Result<Vec<geocode::Candidate>, String> {
    let settings = settings::load(&app)?;
    geocoder.search(&settings, &query).await
}

/// Preview (dry run) or apply GPS to every eligible photo. Emits "apply-progress" events: [done, total].
#[tauri::command]
async fn apply_gps(app: tauri::AppHandle, request: apply::ApplyRequest) -> Result<apply::ApplyResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let progress = |done: usize, total: usize| {
            let _ = app.emit("apply-progress", (done, total));
        };
        apply::run(&request, &progress)
    })
    .await
    .map_err(|e| format!("apply task failed: {e}"))?
}

/// Single-file mode: current GPS, date taken, camera and a preview of one photo.
#[tauri::command]
async fn inspect_photo(path: String) -> Result<inspect::PhotoDetails, String> {
    tauri::async_runtime::spawn_blocking(move || inspect::inspect(&path))
        .await
        .map_err(|e| format!("inspect task failed: {e}"))?
}

/// Everything the ⚙ Settings dialog shows.
#[derive(serde::Serialize)]
struct SettingsView {
    settings: settings::Settings,
    /// Full path of settings.json (for "Open settings folder").
    path: String,
    /// Number of cached searches.
    cache_entries: usize,
    default_nominatim_url: &'static str,
}

/// Current settings for the Settings dialog (creates settings.json if missing).
#[tauri::command]
fn get_settings(app: tauri::AppHandle, geocoder: tauri::State<'_, geocode::Geocoder>) -> Result<SettingsView, String> {
    Ok(SettingsView {
        settings: settings::load(&app)?,
        path: settings::settings_path(&app)?.display().to_string(),
        cache_entries: geocoder.cache_len(),
        default_nominatim_url: settings::DEFAULT_NOMINATIM_URL,
    })
}

/// Validate and save settings from the dialog. Returns the saved (cleaned-up) values.
#[tauri::command]
fn save_settings(app: tauri::AppHandle, settings: settings::Settings) -> Result<settings::Settings, String> {
    settings::save(&app, settings)
}

/// Empty the place-search cache. Returns how many searches were removed.
#[tauri::command]
fn clear_search_cache(geocoder: tauri::State<'_, geocode::Geocoder>) -> usize {
    geocoder.clear_cache()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let cache_dir = app.path().app_cache_dir()?;
            app.manage(geocode::Geocoder::new(cache_dir.join("geocode-cache.json")));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            scan_folder,
            geocode,
            apply_gps,
            inspect_photo,
            get_settings,
            save_settings,
            clear_search_cache
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
