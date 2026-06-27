use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use sqlx::SqlitePool;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};

// 防抖间隔：最后一次文件变化后等 3 秒再触发扫描
const DEBOUNCE: Duration = Duration::from_secs(3);

/// 启动库根目录监听。返回 watcher（需保活）。
pub fn start_watcher(
    app: AppHandle,
    pool: SqlitePool,
    lib_id: i64,
    root: PathBuf,
) -> notify::Result<RecommendedWatcher> {
    let pending = Arc::new(Mutex::new(Option::<Instant>::None));
    let pending_w = pending.clone();

    let watcher = RecommendedWatcher::new(
        move |res: notify::Result<notify::Event>| {
            if let Ok(ev) = res {
                let relevant = matches!(
                    ev.kind,
                    EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
                );
                if !relevant {
                    return;
                }
                let mut p = pending_w.lock().unwrap();
                *p = Some(Instant::now());
            }
        },
        Config::default(),
    )?;

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
            let _ = app2.emit("library://auto-scanning", ());
            // 用 tauri 的 async runtime 执行增量扫描
            let app3 = app2.clone();
            let pool3 = pool2.clone();
            let root3 = root2.clone();
            tauri::async_runtime::spawn(async move {
                match crate::indexer::scan_into(&pool3, lib_id, &root3).await {
                    Ok(report) => {
                        let _ = app3.emit("library://auto-scanned", &report);
                    }
                    Err(e) => {
                        let _ = app3.emit("library://auto-scan-error", e.to_string());
                    }
                }
            });
        }
    });

    let mut w = watcher;
    w.watch(Path::new(&root), RecursiveMode::Recursive)?;
    Ok(w)
}
