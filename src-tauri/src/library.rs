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

/// 全局搜索结果（跨包/跨目录）
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
    pub directory_id: Option<i64>,
}

/// 全局跨包/跨目录搜索：按文件名模糊匹配 + 可选 kind 过滤
/// 同时覆盖两级视图文件（package_id）和树视图文件（directory_id）
#[tauri::command]
pub async fn search_files(
    query: String,
    kind: Option<String>,
    pool: State<'_, SqlitePool>,
) -> Result<Vec<SearchHit>, AppError> {
    let like = format!("%{}%", query.trim());
    // 字段：id, name, ext, kind, bytes, rel_path, package_id, directory_id,
    //       pkg_path, dir_path, package_name, category_name, library_root
    let rows: Vec<(
        i64, String, String, String, i64, String,
        Option<i64>, Option<i64>, Option<String>, Option<String>,
        String, String, Option<String>,
    )> = match &kind {
        Some(k) if !k.is_empty() => sqlx::query_as(
            "SELECT f.id, f.name, f.ext, f.kind, f.bytes, f.rel_path,
                    f.package_id, f.directory_id,
                    p.path, d.path,
                    COALESCE(p.name, d.name), COALESCE(c.name, ''),
                    l.root_path
             FROM files f
             LEFT JOIN packages p ON p.id=f.package_id
             LEFT JOIN categories c ON c.id=p.category_id
             LEFT JOIN directories d ON d.id=f.directory_id
             LEFT JOIN libraries l ON l.id = COALESCE(d.library_id, c.library_id)
             WHERE f.deleted=0 AND f.kind=? AND f.name LIKE ?
             ORDER BY f.name LIMIT 500",
        )
        .bind(k)
        .bind(&like)
        .fetch_all(&*pool)
        .await?,
        _ => sqlx::query_as(
            "SELECT f.id, f.name, f.ext, f.kind, f.bytes, f.rel_path,
                    f.package_id, f.directory_id,
                    p.path, d.path,
                    COALESCE(p.name, d.name), COALESCE(c.name, ''),
                    l.root_path
             FROM files f
             LEFT JOIN packages p ON p.id=f.package_id
             LEFT JOIN categories c ON c.id=p.category_id
             LEFT JOIN directories d ON d.id=f.directory_id
             LEFT JOIN libraries l ON l.id = COALESCE(d.library_id, c.library_id)
             WHERE f.deleted=0 AND f.name LIKE ?
             ORDER BY f.name LIMIT 500",
        )
        .bind(&like)
        .fetch_all(&*pool)
        .await?,
    };
    Ok(rows
        .into_iter()
        .map(|(id, name, ext, kind, bytes, rel, pkg_id, dir_id, pkg_path, dir_path, pkg_name, cat_name, root)| {
            // 拼绝对路径：树视图=库根/dir_path/rel；两级视图=pkg_path/rel（统一正斜杠）
            let abs_path = if let (Some(dp), Some(rt)) = (dir_path, &root) {
                format!("{}/{}/{}", rt.replace('\\', "/"), dp, rel)
            } else {
                format!("{}/{}", pkg_path.unwrap_or_default().replace('\\', "/"), rel)
            };
            SearchHit {
                id,
                name,
                ext,
                kind,
                bytes,
                abs_path,
                package_name: pkg_name,
                category_name: cat_name,
                package_id: pkg_id.unwrap_or(0),
                directory_id: dir_id,
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
    let tree_files: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM files WHERE package_id=0 AND deleted=0")
            .fetch_one(&*pool)
            .await?;
    // 有目录结构但没有树文件 → 迁移后需要重扫
    Ok(dir_count.0 > 0 && tree_files.0 == 0)
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

