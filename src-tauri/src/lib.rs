mod geocode;
mod scan;
mod settings;

use tauri::Manager;

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

/// Where settings.json lives, so the UI can tell the user.
#[tauri::command]
fn settings_location(app: tauri::AppHandle) -> Result<String, String> {
    settings::load(&app)?; // make sure the file exists
    Ok(settings::settings_path(&app)?.display().to_string())
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
        .invoke_handler(tauri::generate_handler![scan_folder, geocode, settings_location])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
