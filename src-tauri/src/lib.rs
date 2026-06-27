mod asset_types;
mod db;
mod indexer;
mod library;
mod scanner;
use tauri::Manager;
use tauri_plugin_log::{Target, TargetKind};

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(
            tauri_plugin_log::Builder::new()
                .targets([
                    Target::new(TargetKind::Stdout),
                    Target::new(TargetKind::Webview),
                ])
                .build(),
        )
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let pool = tauri::async_runtime::block_on(async {
                let pool = db::connect().await.expect("db connect");
                db::migrate(&pool).await.expect("db migrate");
                pool
            });
            app.manage(pool);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            library::add_library,
            library::list_libraries,
            library::scan_library_full,
            library::get_categories,
            library::get_packages,
            library::get_package_files
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
