use crate::asset_types::Registry;
use crate::scanner::{self, ScanEntry};
use serde::Serialize;
use sqlx::SqlitePool;
use std::collections::{HashMap, HashSet};
use std::path::Path;

#[derive(Debug, Serialize)]
pub struct ScanReport {
    pub new: u64,
    pub updated: u64,
    pub deleted: u64,
    pub total_files: u64,
    pub duration_ms: u128,
    pub errors: Vec<String>,
    pub unknown_extensions: Vec<(String, u64)>,
}

/// 确保 分类/包 行存在，返回 (category_id, package_id)。
async fn ensure_cat_pkg(
    pool: &SqlitePool,
    lib_id: i64,
    category: &str,
    package: &str,
    pkg_path: &str,
) -> Result<(i64, i64), sqlx::Error> {
    // 分类
    let sort_order: i64 = category
        .split('_')
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(999);
    sqlx::query(
        "INSERT INTO categories(library_id,name,sort_order) VALUES(?,?,?)
         ON CONFLICT(library_id,name) DO NOTHING",
    )
    .bind(lib_id)
    .bind(category)
    .bind(sort_order)
    .execute(pool)
    .await?;
    let (cat_id,): (i64,) =
        sqlx::query_as("SELECT id FROM categories WHERE library_id=? AND name=?")
            .bind(lib_id)
            .bind(category)
            .fetch_one(pool)
            .await?;
    // 包
    sqlx::query(
        "INSERT INTO packages(category_id,name,path) VALUES(?,?,?)
         ON CONFLICT(category_id,name) DO UPDATE SET path=excluded.path",
    )
    .bind(cat_id)
    .bind(package)
    .bind(pkg_path)
    .execute(pool)
    .await?;
    let (pkg_id,): (i64,) = sqlx::query_as("SELECT id FROM packages WHERE category_id=? AND name=?")
        .bind(cat_id)
        .bind(package)
        .fetch_one(pool)
        .await?;
    Ok((cat_id, pkg_id))
}

/// 全量/增量扫描入库。
pub async fn scan_into(
    pool: &SqlitePool,
    lib_id: i64,
    root: &Path,
) -> Result<ScanReport, sqlx::Error> {
    let start = std::time::Instant::now();
    let registry = Registry::load(pool).await?;
    let mut unknown: HashMap<String, u64> = HashMap::new();
    let entries = scanner::scan_library(root);

    let mut new = 0u64;
    let mut updated = 0u64;
    let mut deleted = 0u64;
    let mut total_written = 0u64;
    let mut seen: HashSet<(i64, String)> = HashSet::new();

    // 按 (category,package) 分组，减少 ensure 查询
    let mut groups: HashMap<(String, String), Vec<ScanEntry>> = HashMap::new();
    for e in entries {
        groups
            .entry((e.category.clone(), e.package.clone()))
            .or_default()
            .push(e);
    }

    for ((cat, pkg), files) in groups {
        let pkg_path = root
            .join(&cat)
            .join(&pkg)
            .to_string_lossy()
            .replace('\\', "/");
        let (_cat_id, pkg_id) = ensure_cat_pkg(pool, lib_id, &cat, &pkg, &pkg_path).await?;
        let mut total_bytes = 0i64;
        let mut file_count = 0i64;
        let mut has_zip = 0i64;
        for f in &files {
            let kind = if registry.kind_for(&f.ext) == "other" && !f.ext.is_empty() {
                *unknown.entry(f.ext.clone()).or_insert(0) += 1;
                "other"
            } else {
                registry.kind_for(&f.ext)
            };
            // 先查是否已存在，用于区分 new/updated
            let existing: Option<(i64,)> =
                sqlx::query_as("SELECT id FROM files WHERE package_id=? AND rel_path=?")
                    .bind(pkg_id)
                    .bind(&f.rel_path)
                    .fetch_optional(pool)
                    .await?;
            sqlx::query(
                "INSERT INTO files(package_id,rel_path,name,ext,kind,bytes,modified_at,deleted)
                 VALUES(?,?,?,?,?,?,?,0)
                 ON CONFLICT(package_id,rel_path) DO UPDATE SET
                   name=excluded.name, ext=excluded.ext, kind=excluded.kind,
                   bytes=excluded.bytes, modified_at=excluded.modified_at, deleted=0",
            )
            .bind(pkg_id)
            .bind(&f.rel_path)
            .bind(&f.name)
            .bind(&f.ext)
            .bind(kind)
            .bind(f.bytes as i64)
            .bind(f.modified_at)
            .execute(pool)
            .await?;
            match existing {
                Some(_) => updated += 1,
                None => new += 1,
            }
            total_written += 1;
            seen.insert((pkg_id, f.rel_path.clone()));
            total_bytes += f.bytes as i64;
            file_count += 1;
            if f.ext == "zip" || f.ext == "7z" || f.ext == "rar" {
                has_zip = 1;
            }
        }
        // 标记消失文件为软删除并计数
        let existing_rows: Vec<(i64, String)> =
            sqlx::query_as("SELECT id, rel_path FROM files WHERE package_id=? AND deleted=0")
                .bind(pkg_id)
                .fetch_all(pool)
                .await?;
        for (id, rp) in existing_rows {
            if !seen.contains(&(pkg_id, rp)) {
                sqlx::query("UPDATE files SET deleted=1 WHERE id=?")
                    .bind(id)
                    .execute(pool)
                    .await?;
                deleted += 1;
            }
        }
        sqlx::query("UPDATE packages SET file_count=?, total_bytes=?, has_zip=? WHERE id=?")
            .bind(file_count)
            .bind(total_bytes)
            .bind(has_zip)
            .bind(pkg_id)
            .execute(pool)
            .await?;
    }

    Ok(ScanReport {
        new,
        updated,
        deleted,
        total_files: total_written,
        duration_ms: start.elapsed().as_millis(),
        errors: vec![],
        unknown_extensions: unknown.into_iter().collect(),
    })
}
