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
    // 崩溃日志：在一切初始化之前装 panic hook，
    // 把任何 panic（包括 setup 阶段的 expect）写到 crash.log，便于定位启动闪退。
    install_panic_hook();

    // 日志目标：debug 构建写文件 + 控制台（开发排查用）；release 仅控制台 Warning+。
    // logs/ 目录在 .gitignore 忽略，不污染 git/打包。
    let log_targets: Vec<Target> = {
        let mut v = vec![Target::new(TargetKind::Stdout)];
        #[cfg(debug_assertions)]
        {
            let folder = Target::new(TargetKind::Folder {
                path: std::env::current_dir().unwrap_or_default(),
                file_name: Some("app.log".into()),
            })
            .filter(|m| m.level() <= log::LevelFilter::Debug);
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

/// 安装全局 panic hook：把崩溃信息写到 %APPDATA%/com.xiaoke.vault/crash.log，
/// 便于诊断启动闪退（绕过 tauri_plugin_log，确保最早期 panic 也能记录）。
fn install_panic_hook() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        // 写崩溃日志到应用数据目录
        if let Some(data) = dirs::data_dir() {
            let log_path = data.join("com.xiaoke.vault").join("crash.log");
            let _ = std::fs::create_dir_all(log_path.parent().unwrap_or(std::path::Path::new(".")));
            let msg = format!(
                "[{}] PANIC: {}\nbacktrace:\n{}\n\n",
                chrono::Utc::now().format("%Y-%m-%d %H:%M:%S"),
                info,
                std::backtrace::Backtrace::force_capture()
            );
            // 追加写，保留历史崩溃记录
            let _ = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&log_path)
                .and_then(|mut f| std::io::Write::write_all(&mut f, msg.as_bytes()));
        }
        // 继续调用默认 hook（输出到 stderr）
        default_hook(info);
    }));
}
