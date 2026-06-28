use crate::error::AppError;
use crate::indexer::{self, ScanReport};
use serde::Serialize;
use sqlx::SqlitePool;
use std::path::PathBuf;
use tauri::State;

#[derive(Debug, Serialize)]
pub struct Library {
    pub id: i64,
    pub name: String,
    pub root_path: String,
}

/// 库内文件节点（用于文件网格展示）
#[derive(Debug, Serialize)]
pub struct FileNode {
    pub id: i64,
    pub rel_path: String,
    pub name: String,
    pub ext: String,
    pub kind: String,
    pub bytes: i64,
    pub abs_path: String,
}

/// 全局搜索结果（跨目录）
#[derive(Debug, Serialize)]
pub struct SearchHit {
    pub id: i64,
    pub name: String,
    pub ext: String,
    pub kind: String,
    pub bytes: i64,
    pub abs_path: String,
    pub directory_id: Option<i64>,
    /// 文件所在目录的相对路径（库根下的路径，用于结果展示）
    pub directory_path: String,
}

/// 全局跨目录搜索：按文件名模糊匹配 + 可选 kind 过滤
#[tauri::command]
pub async fn search_files(
    query: String,
    kind: Option<String>,
    pool: State<'_, SqlitePool>,
) -> Result<Vec<SearchHit>, AppError> {
    let like = format!("%{}%", query.trim());
    // 字段：id, name, ext, kind, bytes, rel_path, directory_id, dir_path, library_root
    let rows: Vec<(
        i64, String, String, String, i64, String,
        Option<i64>, Option<String>, Option<String>,
    )> = match &kind {
        Some(k) if !k.is_empty() => sqlx::query_as(
            "SELECT f.id, f.name, f.ext, f.kind, f.bytes, f.rel_path,
                    f.directory_id, d.path, l.root_path
             FROM files f
             LEFT JOIN directories d ON d.id=f.directory_id
             LEFT JOIN libraries l ON l.id=d.library_id
             WHERE f.deleted=0 AND f.kind=? AND f.name LIKE ?
             ORDER BY f.name LIMIT 500",
        )
        .bind(k)
        .bind(&like)
        .fetch_all(&*pool)
        .await?,
        _ => sqlx::query_as(
            "SELECT f.id, f.name, f.ext, f.kind, f.bytes, f.rel_path,
                    f.directory_id, d.path, l.root_path
             FROM files f
             LEFT JOIN directories d ON d.id=f.directory_id
             LEFT JOIN libraries l ON l.id=d.library_id
             WHERE f.deleted=0 AND f.name LIKE ?
             ORDER BY f.name LIMIT 500",
        )
        .bind(&like)
        .fetch_all(&*pool)
        .await?,
    };
    Ok(rows
        .into_iter()
        .map(|(id, name, ext, kind, bytes, rel, dir_id, dir_path, root)| {
            // 拼绝对路径：库根/dir_path/rel（统一正斜杠）
            let rt = root.unwrap_or_default().replace('\\', "/");
            let dp = dir_path.unwrap_or_default();
            let abs_path = format!("{}/{}/{}", rt, dp, rel);
            SearchHit {
                id,
                name,
                ext,
                kind,
                bytes,
                abs_path,
                directory_id: dir_id,
                directory_path: dp,
            }
        })
        .collect())
}

#[tauri::command]
pub async fn add_library(
    name: String,
    root_path: String,
    pool: State<'_, SqlitePool>,
) -> Result<Library, AppError> {
    let now = chrono::Utc::now().timestamp();
    sqlx::query("INSERT INTO libraries(name,root_path,created_at) VALUES(?,?,?)")
        .bind(&name)
        .bind(&root_path)
        .bind(now)
        .execute(&*pool)
        .await?;
    let (id,): (i64,) = sqlx::query_as("SELECT id FROM libraries WHERE root_path=?")
        .bind(&root_path)
        .fetch_one(&*pool)
        .await?;
    Ok(Library {
        id,
        name,
        root_path,
    })
}

#[tauri::command]
pub async fn list_libraries(pool: State<'_, SqlitePool>) -> Result<Vec<Library>, AppError> {
    let rows: Vec<(i64, String, String)> =
        sqlx::query_as("SELECT id,name,root_path FROM libraries ORDER BY id")
            .fetch_all(&*pool)
            .await?;
    Ok(rows
        .into_iter()
        .map(|(id, name, root_path)| Library {
            id,
            name,
            root_path,
        })
        .collect())
}

/// 检测是否需要重扫（迁移后树文件被清空，但 directories 还在）。
/// 启动时调用：若 directories 有记录但 files 表无 package_id=0 的树文件，返回 true。
#[tauri::command]
pub async fn needs_rescan(
    lib_id: i64,
    pool: State<'_, SqlitePool>,
) -> Result<bool, AppError> {
    let dir_count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM directories WHERE library_id=?")
            .bind(lib_id)
            .fetch_one(&*pool)
            .await?;
    let file_count: (i64,) =
        sqlx::query_as(
            "SELECT COUNT(*) FROM files f
             JOIN directories d ON d.id=f.directory_id
             WHERE d.library_id=? AND f.deleted=0",
        )
        .bind(lib_id)
        .fetch_one(&*pool)
        .await?;
    // 有目录结构但没有文件 → 需要重扫
    Ok(dir_count.0 > 0 && file_count.0 == 0)
}

#[tauri::command]
pub async fn scan_library_full(
    lib_id: i64,
    pool: State<'_, SqlitePool>,
) -> Result<ScanReport, AppError> {
    let (root,): (String,) = sqlx::query_as("SELECT root_path FROM libraries WHERE id=?")
        .bind(lib_id)
        .fetch_one(&*pool)
        .await?;
    // 统一的目录树扫描（写 directories + files 表）
    let report = indexer::scan_tree_into(&*pool, lib_id, &PathBuf::from(&root)).await?;
    let now = chrono::Utc::now().timestamp();
    sqlx::query("UPDATE libraries SET last_scan_at=? WHERE id=?")
        .bind(now)
        .bind(lib_id)
        .execute(&*pool)
        .await?;
    Ok(report)
}

