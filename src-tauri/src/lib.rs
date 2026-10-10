mod scan;

/// Scan a folder for supported photos. Runs on a background thread so the UI stays responsive.
#[tauri::command]
async fn scan_folder(path: String, recursive: bool) -> Result<scan::ScanResult, String> {
    tauri::async_runtime::spawn_blocking(move || scan::scan(&path, recursive))
        .await
        .map_err(|e| format!("scan task failed: {e}"))?
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![scan_folder])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
