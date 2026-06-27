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

#[derive(Debug, Serialize)]
pub struct Category {
    pub id: i64,
    pub name: String,
    pub sort_order: i64,
    pub package_count: i64,
    pub file_count: i64,
    pub total_bytes: i64,
}

#[derive(Debug, Serialize)]
pub struct PackageSummary {
    pub id: i64,
    pub name: String,
    pub path: String,
    pub file_count: i64,
    pub total_bytes: i64,
    pub has_zip: bool,
    pub license: Option<String>,
}

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

/// 全局搜索结果（跨包）
#[derive(Debug, Serialize)]
pub struct SearchHit {
    pub id: i64,
    pub name: String,
    pub ext: String,
    pub kind: String,
    pub bytes: i64,
    pub abs_path: String,
    pub package_name: String,
    pub category_name: String,
    pub package_id: i64,
}

/// 全局跨包搜索：按文件名模糊匹配 + 可选 kind 过滤
#[tauri::command]
pub async fn search_files(
    query: String,
    kind: Option<String>,
    pool: State<'_, SqlitePool>,
) -> Result<Vec<SearchHit>, AppError> {
    let like = format!("%{}%", query.trim());
    let rows: Vec<(i64, String, String, String, i64, String, String, String, i64)> = match &kind {
        Some(k) if !k.is_empty() => sqlx::query_as(
            "SELECT f.id, f.name, f.ext, f.kind, f.bytes, p.path || '/' || f.rel_path, p.name, c.name, p.id
             FROM files f
             JOIN packages p ON p.id=f.package_id
             JOIN categories c ON c.id=p.category_id
             WHERE f.deleted=0 AND f.kind=? AND f.name LIKE ?
             ORDER BY f.name LIMIT 500",
        )
        .bind(k)
        .bind(&like)
        .fetch_all(&*pool)
        .await?,
        _ => sqlx::query_as(
            "SELECT f.id, f.name, f.ext, f.kind, f.bytes, p.path || '/' || f.rel_path, p.name, c.name, p.id
             FROM files f
             JOIN packages p ON p.id=f.package_id
             JOIN categories c ON c.id=p.category_id
             WHERE f.deleted=0 AND f.name LIKE ?
             ORDER BY f.name LIMIT 500",
        )
        .bind(&like)
        .fetch_all(&*pool)
        .await?,
    };
    Ok(rows
        .into_iter()
        .map(
            |(id, name, ext, kind, bytes, abs_path, package_name, category_name, package_id)| SearchHit {
                id, name, ext, kind, bytes, abs_path, package_name, category_name, package_id,
            },
        )
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

#[tauri::command]
pub async fn scan_library_full(
    lib_id: i64,
    pool: State<'_, SqlitePool>,
) -> Result<ScanReport, AppError> {
    let (root,): (String,) = sqlx::query_as("SELECT root_path FROM libraries WHERE id=?")
        .bind(lib_id)
        .fetch_one(&*pool)
        .await?;
    let report = indexer::scan_into(&*pool, lib_id, &PathBuf::from(&root))
        .await?;
    let now = chrono::Utc::now().timestamp();
    sqlx::query("UPDATE libraries SET last_scan_at=? WHERE id=?")
        .bind(now)
        .bind(lib_id)
        .execute(&*pool)
        .await?;
    // 同时跑目录树扫描（写 directories 表）
    if let Err(e) = indexer::scan_tree_into(&*pool, lib_id, &PathBuf::from(&root)).await {
        log::warn!("[scan] 目录树扫描失败（不影响主扫描）：{e}");
    }
    Ok(report)
}

#[tauri::command]
pub async fn get_categories(
    lib_id: i64,
    pool: State<'_, SqlitePool>,
) -> Result<Vec<Category>, AppError> {
    let rows: Vec<(i64, String, i64, i64, i64, i64)> = sqlx::query_as(
        "SELECT c.id,c.name,c.sort_order,
           (SELECT COUNT(*) FROM packages p WHERE p.category_id=c.id),
           (SELECT COUNT(*) FROM files f JOIN packages p ON p.id=f.package_id WHERE p.category_id=c.id AND f.deleted=0),
           (SELECT COALESCE(SUM(f.bytes),0) FROM files f JOIN packages p ON p.id=f.package_id WHERE p.category_id=c.id AND f.deleted=0)
         FROM categories c WHERE c.library_id=? ORDER BY c.sort_order",
    )
    .bind(lib_id)
    .fetch_all(&*pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(id, name, sort, pc, fc, tb)| Category {
            id,
            name,
            sort_order: sort,
            package_count: pc,
            file_count: fc,
            total_bytes: tb,
        })
        .collect())
}

#[tauri::command]
pub async fn get_packages(
    category_id: i64,
    pool: State<'_, SqlitePool>,
) -> Result<Vec<PackageSummary>, AppError> {
    let rows: Vec<(i64, String, String, i64, i64, i64, Option<String>)> = sqlx::query_as(
        "SELECT id,name,path,file_count,total_bytes,has_zip,license FROM packages
         WHERE category_id=? ORDER BY name",
    )
    .bind(category_id)
    .fetch_all(&*pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(id, name, path, fc, tb, hz, lic)| PackageSummary {
            id,
            name,
            path,
            file_count: fc,
            total_bytes: tb,
            has_zip: hz != 0,
            license: lic,
        })
        .collect())
}

#[tauri::command]
pub async fn get_package_files(
    pkg_id: i64,
    pool: State<'_, SqlitePool>,
) -> Result<Vec<FileNode>, AppError> {
    let rows: Vec<(i64, String, String, String, String, i64, String)> = sqlx::query_as(
        "SELECT f.id,f.rel_path,f.name,f.ext,f.kind,f.bytes,p.path
         FROM files f JOIN packages p ON p.id=f.package_id
         WHERE f.package_id=? AND f.deleted=0 ORDER BY f.rel_path",
    )
    .bind(pkg_id)
    .fetch_all(&*pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(id, rel, name, ext, kind, bytes, pkg_path)| FileNode {
            id,
            rel_path: rel.clone(),
            name,
            ext,
            kind,
            bytes,
            abs_path: format!("{}/{}", pkg_path, rel),
        })
        .collect())
}
