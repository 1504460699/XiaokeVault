use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use sqlx::SqlitePool;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};

// 防抖间隔：最后一次文件变化后等 3 秒再触发扫描
const DEBOUNCE: Duration = Duration::from_secs(3);

/// 启动库根目录监听。返回 watcher（调用方必须保活，否则监听立即停止）。
pub fn start_watcher(
    app: AppHandle,
    pool: SqlitePool,
    lib_id: i64,
    root: PathBuf,
) -> notify::Result<RecommendedWatcher> {
    log::info!("[watcher] start_watcher 入口，lib_id={}, root={}", lib_id, root.display());

    let pending = Arc::new(Mutex::new(Option::<Instant>::None));
    let pending_w = pending.clone();

    let watcher = RecommendedWatcher::new(
        move |res: notify::Result<notify::Event>| {
            match res {
                Ok(ev) => {
                    let relevant = matches!(
                        ev.kind,
                        EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
                    );
                    if !relevant {
                        return;
                    }
                    // 仅记录关心的路径，避免日志爆炸
                    let paths: Vec<String> = ev
                        .paths
                        .iter()
                        .map(|p| p.file_name().and_then(|n| n.to_str()).unwrap_or("?").to_string())
                        .collect();
                    log::debug!("[watcher] 捕获事件 {:?} -> {:?}", ev.kind, paths);
                    let mut p = pending_w.lock().unwrap();
                    *p = Some(Instant::now());
                }
                Err(e) => {
                    log::warn!("[watcher] 监听错误：{e}");
                }
            }
        },
        Config::default(),
    )?;

    let mut w = watcher;
    match w.watch(Path::new(&root), RecursiveMode::Recursive) {
        Ok(()) => log::info!("[watcher] watch() 注册成功"),
        Err(e) => {
            log::error!("[watcher] watch() 注册失败：{e}");
            return Err(e);
        }
    }

    let pending2 = pending.clone();
    let app2 = app.clone();
    let pool2 = pool.clone();
    let root2 = root.clone();
    let stopped = Arc::new(AtomicBool::new(false));
    let stopped2 = stopped.clone();
    app.manage(stopped);

    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_millis(500));
        if stopped2.load(Ordering::SeqCst) {
            log::info!("[watcher] 轮询线程退出");
            break;
        }
        let should_scan = {
            let mut p = pending2.lock().unwrap();
            if let Some(t) = *p {
                if t.elapsed() >= DEBOUNCE {
                    *p = None;
                    true
                } else {
                    false
                }
            } else {
                false
            }
        };
        if should_scan {
            log::info!("[watcher] 防抖结束，触发增量扫描");
            let _ = app2.emit("library://auto-scanning", ());
            // 用 tauri 的 async runtime 执行增量扫描
            let app3 = app2.clone();
            let pool3 = pool2.clone();
            let root3 = root2.clone();
            tauri::async_runtime::spawn(async move {
                match crate::indexer::scan_into(&pool3, lib_id, &root3).await {
                    Ok(report) => {
                        log::info!(
                            "[watcher] 增量扫描完成：新增 {} / 更新 {} / 删除 {} / 耗时 {}ms",
                            report.new,
                            report.updated,
                            report.deleted,
                            report.duration_ms
                        );
                        // 同步刷新目录树（如实反映目录增删改）
                        if let Err(e) =
                            crate::indexer::scan_tree_into(&pool3, lib_id, &root3).await
                        {
                            log::warn!("[watcher] 目录树同步失败：{e}");
                        }
                        let _ = app3.emit("library://auto-scanned", &report);
                    }
                    Err(e) => {
                        log::error!("[watcher] 增量扫描失败：{e}");
                        let _ = app3.emit("library://auto-scan-error", e.to_string());
                    }
                }
            });
        }
    });

    Ok(w)
}
