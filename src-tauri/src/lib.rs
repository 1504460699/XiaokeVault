mod asset_types;
mod db;
mod dedup;
mod exporter;
mod indexer;
mod library;
mod preview;
mod scanner;
mod selection;
mod watcher;
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
            // 应用启动时，若已有库则启动文件监听（自动增量扫描）
            {
                let app_handle = app.handle().clone();
                let res: (i64, String) = tauri::async_runtime::block_on(async {
                    sqlx::query_as("SELECT id, root_path FROM libraries ORDER BY id LIMIT 1")
                        .fetch_one(&pool)
                        .await
                        .unwrap_or((0, String::new()))
                });
                if res.0 != 0 {
                    let _ = watcher::start_watcher(
                        app_handle,
                        pool.clone(),
                        res.0,
                        std::path::PathBuf::from(res.1),
                    );
                }
            }
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
            library::get_package_files,
            library::search_files,
            selection::create_project,
            selection::list_projects,
            selection::set_selection,
            selection::clear_selections,
            selection::get_selected_file_ids,
            selection::get_category_selection_states,
            selection::get_selection_summary,
            exporter::run_export,
            dedup::run_dedup,
            dedup::get_duplicate_groups,
            dedup::remove_duplicate,
            dedup::remove_all_duplicates,
            dedup::dismiss_duplicate_group,
            preview::get_model_glb,
            preview::get_thumbnail,
            asset_types::list_asset_types,
            asset_types::upsert_asset_type,
            asset_types::delete_asset_type,
            asset_types::reclassify_all
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
