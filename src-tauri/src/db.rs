use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;
use std::path::PathBuf;
use std::str::FromStr;

/// 返回数据库文件路径：<app_data>/com.xiaoke.tauri-app/index.db
pub fn db_path() -> PathBuf {
    let mut p = dirs::data_dir().expect("no data dir");
    p.push("com.xiaoke.tauri-app");
    std::fs::create_dir_all(&p).expect("create app data dir");
    p.push("index.db");
    p
}

/// 创建连接池（启用外键）。
pub async fn connect() -> Result<SqlitePool, sqlx::Error> {
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
    Ok(())
}
