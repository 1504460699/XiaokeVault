mod app_log;
mod asset_types;
mod db;
mod error;
mod exporter;
mod indexer;
mod library;
mod preview;
mod selection;
mod watcher;
mod tree_scanner;
mod tree;
use tauri::Manager;
use tauri_plugin_log::{Target, TargetKind};

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // 自定义文件日志：写到 %APPDATA%/com.xiaoke.vault/app.log（固定位置，与 DB 同目录）。
    // panic hook 确保任何崩溃也能记录。
    app_log::install_panic_hook();
    alog_info!("app", "应用启动");

    // tauri_plugin_log 仅保留 stdout（控制台），文件日志由 app_log 负责
    let log_targets: Vec<Target> = vec![Target::new(TargetKind::Stdout)];
    let log_builder = tauri_plugin_log::Builder::new()
        .targets(log_targets)
        .level(log::LevelFilter::Warn);

    tauri::Builder::default()
        .plugin(log_builder.build())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            alog_info!("app", "setup 开始：连接数据库 + 迁移");
            let pool = tauri::async_runtime::block_on(async {
                let pool = match db::connect().await {
                    Ok(p) => {
                        alog_info!("db", "数据库连接成功");
                        p
                    }
                    Err(e) => {
                        alog_error!("db", "数据库连接失败：{e}");
                        panic!("db connect: {e}");
                    }
                };
                match db::migrate(&pool).await {
                    Ok(_) => alog_info!("db", "迁移完成"),
                    Err(e) => {
                        alog_error!("db", "迁移失败：{e}");
                        panic!("db migrate: {e}");
                    }
                }
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
                    alog_info!("watcher", "发现已有库 id={}，启动文件监听", res.0);
                    match watcher::start_watcher(
                        app_handle,
                        pool.clone(),
                        res.0,
                        std::path::PathBuf::from(&res.1),
                    ) {
                        Ok(w) => {
                            // 关键：必须 manage 保活整个应用生命周期，
                            // 否则 RecommendedWatcher 被 drop 后 OS 级文件监听立即停止。
                            app.manage(w);
                            alog_info!("watcher", "已启动，监听目录：{}", res.1);
                        }
                        Err(e) => {
                            alog_error!("watcher", "启动失败：{e}");
                        }
                    }
                } else {
                    alog_info!("watcher", "未发现已有库，跳过自动监听");
                }
            }
            app.manage(pool);
            alog_info!("app", "setup 完成，应用就绪");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            library::add_library,
            library::list_libraries,
            library::scan_library_full,
            library::needs_rescan,
            library::search_files,
            selection::create_project,
            selection::list_projects,
            selection::set_selection,
            selection::clear_selections,
            selection::get_selected_file_ids,
            selection::get_directory_selection_state,
            selection::get_selection_summary,
            exporter::run_export,
            preview::get_model_glb,
            preview::get_thumbnail,
            tree::get_directory_tree,
            tree::get_directory_files,
            tree::get_subtree_files,
            tree::get_all_library_files,
            asset_types::list_asset_types,
            asset_types::upsert_asset_type,
            asset_types::delete_asset_type,
            asset_types::reclassify_all
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
    alog_info!("app", "应用退出");
}
