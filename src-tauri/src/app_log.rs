//! 应用日志：直接写文件到 %APPDATA%/com.xiaoke.vault/app.log（固定位置，与 DB 同目录）。
//!
//! 设计目标：日志位置稳定、可靠，不依赖 tauri_plugin_log 的 Folder target
//! （后者在 release 下可能写错位置或因路径问题失败）。
//! 同时注册 panic hook，确保任何崩溃也能记录。

use std::fs::OpenOptions;
use std::io::Write;
use std::sync::Mutex;

static LOG_FILE: Mutex<()> = Mutex::new(());

/// 日志文件绝对路径：%APPDATA%/com.xiaoke.vault/app.log
fn log_path() -> std::path::PathBuf {
    let mut p = crate::db::data_root();
    p.push("app.log");
    p
}

/// 写一条日志（追加）。失败静默忽略（日志不应影响主流程）。
pub fn write(level: &str, target: &str, msg: &str) {
    let _guard = LOG_FILE.lock().unwrap_or_else(|e| e.into_inner());
    let line = format!(
        "[{}][{}][{}] {}\n",
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
        level,
        target,
        msg
    );
    if let Ok(mut f) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path())
    {
        let _ = f.write_all(line.as_bytes());
    }
}

/// 安装 panic hook：把崩溃信息写到 app.log（与正常日志同文件，便于统一查看）。
pub fn install_panic_hook() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let location = info
            .location()
            .map(|l| format!("{}:{}", l.file(), l.line()))
            .unwrap_or_default();
        let msg = format!(
            "PANIC at {} | {}",
            location,
            info.payload()
                .downcast_ref::<&str>()
                .copied()
                .or_else(|| info.payload().downcast_ref::<String>().map(|s| s.as_str()))
                .unwrap_or("<non-string panic>")
        );
        write("ERROR", "panic", &msg);
        default_hook(info);
    }));
}

/// 便捷宏：写 INFO 级日志。
#[macro_export]
macro_rules! alog_info {
    ($target:expr, $($arg:tt)*) => {
        $crate::app_log::write("INFO", $target, &format!($($arg)*))
    };
}

/// 便捷宏：写 ERROR 级日志。
#[macro_export]
macro_rules! alog_error {
    ($target:expr, $($arg:tt)*) => {
        $crate::app_log::write("ERROR", $target, &format!($($arg)*))
    };
}

/// 便捷宏：写 DEBUG 级日志。
#[macro_export]
macro_rules! alog_debug {
    ($target:expr, $($arg:tt)*) => {
        $crate::app_log::write("DEBUG", $target, &format!($($arg)*))
    };
}

/// 便捷宏：写 WARN 级日志。
#[macro_export]
macro_rules! alog_warn {
    ($target:expr, $($arg:tt)*) => {
        $crate::app_log::write("WARN", $target, &format!($($arg)*))
    };
}
