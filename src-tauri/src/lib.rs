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
    // 日志目标：debug 构建写文件 + 控制台（开发排查用）；release 仅控制台 Warning+。
    // logs/ 目录在 .gitignore 忽略，不污染 git/打包。
    let log_targets: Vec<Target> = {
        let mut v = vec![Target::new(TargetKind::Stdout)];
        #[cfg(debug_assertions)]
        {
            // debug：额外输出到项目下 logs/ 文件，级别 Debug
            let mut folder = Target::new(TargetKind::Folder {
                path: std::env::current_dir().unwrap_or_default(),
                file_name: Some("app.log".into()),
            });
            // Folder target 默认追加轮转，够用
            folder = folder.filter(|m| m.level() <= log::LevelFilter::Debug);
            v.push(folder);
        }
        v
    };

    let mut log_builder = tauri_plugin_log::Builder::new().targets(log_targets);
    #[cfg(debug_assertions)]
    {
        log_builder = log_builder.level(log::LevelFilter::Debug);
    }
    #[cfg(not(debug_assertions))]
    {
        log_builder = log_builder.level(log::LevelFilter::Warn);
    }

    tauri::Builder::default()
        .plugin(log_builder.build())
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
                            log::info!("[watcher] 已启动，监听目录：{}", res.1);
                        }
                        Err(e) => {
                            log::error!("[watcher] 启动失败：{e}");
                        }
                    }
                } else {
                    log::info!("[watcher] 未发现已有库，跳过自动监听");
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
}
