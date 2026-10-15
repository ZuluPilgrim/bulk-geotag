mod apply;
mod browse;
mod exif_repair;
mod geocode;
mod inspect;
mod log;
mod scan;
mod settings;
mod thumbs;

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

/// Drag and drop: is a dropped path a folder, a photo the app can read, some other file, or missing?
#[tauri::command]
fn path_kind(path: String) -> &'static str {
    let p = std::path::Path::new(&path);
    if p.is_dir() {
        "folder"
    } else if p.is_file() && scan::is_supported(p) {
        "photo"
    } else if p.exists() {
        "other"
    } else {
        "missing"
    }
}

/// Inspector: the subfolders and photos of one folder (a file lists its own folder).
#[tauri::command]
async fn list_folder(path: String) -> Result<browse::FolderListing, String> {
    tauri::async_runtime::spawn_blocking(move || browse::list_folder(&path))
        .await
        .map_err(|e| format!("list task failed: {e}"))?
}

/// Inspector: a thumbnail for one photo (embedded EXIF thumbnail, else generated and cached).
#[tauri::command]
async fn photo_thumbnail(app: tauri::AppHandle, path: String, large: Option<bool>) -> Result<Option<thumbs::Thumb>, String> {
    let cache = thumbs_dir(&app)?;
    let large = large.unwrap_or(false);
    tauri::async_runtime::spawn_blocking(move || thumbs::thumbnail(std::path::Path::new(&path), &cache, large))
        .await
        .map_err(|e| format!("thumbnail task failed: {e}"))?
}

/// Folder holding generated thumbnails.
fn thumbs_dir(app: &tauri::AppHandle) -> Result<std::path::PathBuf, String> {
    Ok(app.path().app_cache_dir().map_err(|e| format!("no cache folder: {e}"))?.join("thumbs"))
}

/// Everything the ⚙ Settings dialog shows.
#[derive(serde::Serialize)]
struct SettingsView {
    settings: settings::Settings,
    /// Full path of settings.json (for "Open settings folder").
    path: String,
    /// Number of cached searches.
    cache_entries: usize,
    /// Number of generated thumbnails in the cache.
    cache_thumbnails: usize,
    default_nominatim_url: &'static str,
}

/// Current settings for the Settings dialog (creates settings.json if missing).
#[tauri::command]
fn get_settings(app: tauri::AppHandle, geocoder: tauri::State<'_, geocode::Geocoder>) -> Result<SettingsView, String> {
    Ok(SettingsView {
        settings: settings::load(&app)?,
        path: settings::settings_path(&app)?.display().to_string(),
        cache_entries: geocoder.cache_len(),
        cache_thumbnails: thumbs::cache_count(&thumbs_dir(&app)?),
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

/// Delete the generated thumbnails. Returns how many were removed.
#[tauri::command]
fn clear_thumbnail_cache(app: tauri::AppHandle) -> Result<usize, String> {
    Ok(thumbs::clear_cache(&thumbs_dir(&app)?))
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
            clear_search_cache,
            path_kind,
            list_folder,
            photo_thumbnail,
            clear_thumbnail_cache
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::path_kind;

    #[test]
    fn dropped_paths_are_classified() {
        let data = concat!(env!("CARGO_MANIFEST_DIR"), "/test-data");
        assert_eq!(path_kind(data.into()), "folder");
        assert_eq!(path_kind(format!("{data}/exif-details.jpg")), "photo");
        assert_eq!(path_kind(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml").into()), "other");
        assert_eq!(path_kind(format!("{data}/no-such-file.jpg")), "missing");
    }
}
