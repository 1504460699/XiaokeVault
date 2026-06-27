use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;
use std::path::PathBuf;
use std::str::FromStr;

/// 应用数据目录名（变更：tauri-app → vault）。
/// 旧目录 com.xiaoke.tauri-app 会在 connect() 时自动迁移到新目录。
pub const DATA_DIR: &str = "com.xiaoke.vault";
const LEGACY_DATA_DIR: &str = "com.xiaoke.tauri-app";

/// 返回数据库文件路径：<app_data>/com.xiaoke.vault/index.db
pub fn db_path() -> PathBuf {
    let mut p = dirs::data_dir().expect("no data dir");
    p.push(DATA_DIR);
    std::fs::create_dir_all(&p).expect("create app data dir");
    p.push("index.db");
    p
}

/// 应用数据根目录（供缩略图/回收站等共用）。
pub fn data_root() -> PathBuf {
    let mut p = dirs::data_dir().expect("no data dir");
    p.push(DATA_DIR);
    std::fs::create_dir_all(&p).expect("create app data dir");
    p
}

/// 一次性迁移：若新目录为空而旧目录存在，把旧目录整体搬到新目录。
/// 保留已扫描的 index.db、缩略图缓存、去重复份等。
pub fn migrate_legacy_data() {
    let new_root = data_root();
    let mut old = dirs::data_dir().expect("no data dir");
    old.push(LEGACY_DATA_DIR);
    if !old.exists() {
        return;
    }
    // 新目录若已有 index.db，说明已是新数据，不覆盖
    if new_root.join("index.db").exists() {
        return;
    }
    log::info!("[db] 迁移旧数据目录 {} → {}", old.display(), new_root.display());
    // 新目录此时一般只有刚 create_dir_all 建的空壳，先删掉再整体 rename
    let _ = std::fs::remove_dir_all(&new_root);
    if let Err(e) = std::fs::rename(&old, &new_root) {
        // rename 失败（跨卷等），回退到递归拷贝
        log::warn!("[db] rename 失败({e})，尝试递归拷贝");
        if let Err(e) = copy_dir_recursive(&old, &new_root) {
            log::error!("[db] 迁移失败：{e}");
        }
    }
}

fn copy_dir_recursive(src: &std::path::Path, dst: &std::path::Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if from.is_dir() {
            copy_dir_recursive(&from, &to)?;
        } else {
            std::fs::copy(&from, &to)?;
        }
    }
    Ok(())
}

/// 创建连接池（启用外键）。
/// 注意：必须先迁移旧目录数据，再创建新库，否则空库会被误判为「已存在」而跳过迁移。
pub async fn connect() -> Result<SqlitePool, sqlx::Error> {
    migrate_legacy_data();
    let path = db_path();
    let opts = SqliteConnectOptions::from_str(&format!("sqlite://{}", path.display()))?
        .create_if_missing(true)
        .foreign_keys(true);
    SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(opts)
        .await
}

/// 执行初始建表迁移。
pub async fn migrate(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::query(include_str!("../migrations/0001_init.sql"))
        .execute(pool)
        .await?;
    sqlx::query(include_str!("../migrations/0002_projects_selections.sql"))
        .execute(pool)
        .await?;
    sqlx::query(include_str!("../migrations/0003_dedup.sql"))
        .execute(pool)
        .await?;
    Ok(())
}
