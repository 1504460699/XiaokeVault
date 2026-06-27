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

/// 全量/增量扫描入库。
/// 性能要点：整个库包在单个事务里；用一次性批量查询替代逐行 SELECT/INSERT 往返。
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

    // 按 (category,package) 分组，减少 ensure 查询
    let mut groups: HashMap<(String, String), Vec<ScanEntry>> = HashMap::new();
    for e in entries {
        groups
            .entry((e.category.clone(), e.package.clone()))
            .or_default()
            .push(e);
    }

    // 整个扫描作为一个事务，避免每行提交开销
    let mut tx = pool.begin().await?;

    for ((cat, pkg), files) in groups {
        let pkg_path = root
            .join(&cat)
            .join(&pkg)
            .to_string_lossy()
            .replace('\\', "/");
        let (_cat_id, pkg_id) = ensure_cat_pkg_tx(&mut tx, lib_id, &cat, &pkg, &pkg_path).await?;
        let mut total_bytes = 0i64;
        let mut file_count = 0i64;
        let mut has_zip = 0i64;

        // 一次性取该包已存在的 rel_path 集合，用于区分 new/updated 并检测消失
        let existing: HashSet<String> =
            sqlx::query_scalar("SELECT rel_path FROM files WHERE package_id=? AND deleted=0")
                .bind(pkg_id)
                .fetch_all(&mut *tx)
                .await?
                .into_iter()
                .collect();

        let mut seen: HashSet<String> = HashSet::new();
        for f in &files {
            let kind = if registry.kind_for(&f.ext) == "other" && !f.ext.is_empty() {
                *unknown.entry(f.ext.clone()).or_insert(0) += 1;
                "other"
            } else {
                registry.kind_for(&f.ext)
            };
            let existed = existing.contains(&f.rel_path);
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
            .execute(&mut *tx)
            .await?;
            if existed {
                updated += 1;
            } else {
                new += 1;
            }
            total_written += 1;
            seen.insert(f.rel_path.clone());
            total_bytes += f.bytes as i64;
            file_count += 1;
            if f.ext == "zip" || f.ext == "7z" || f.ext == "rar" {
                has_zip = 1;
            }
        }

        // 标记消失文件为软删除并计数
        for rp in existing.iter() {
            if !seen.contains(rp) {
                sqlx::query("UPDATE files SET deleted=1 WHERE package_id=? AND rel_path=?")
                    .bind(pkg_id)
                    .bind(rp)
                    .execute(&mut *tx)
                    .await?;
                deleted += 1;
            }
        }

        sqlx::query("UPDATE packages SET file_count=?, total_bytes=?, has_zip=? WHERE id=?")
            .bind(file_count)
            .bind(total_bytes)
            .bind(has_zip)
            .bind(pkg_id)
            .execute(&mut *tx)
            .await?;
    }

    tx.commit().await?;

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

/// 事务版 ensure_cat_pkg：确保 分类/包 行存在，返回 (category_id, package_id)。
async fn ensure_cat_pkg_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    lib_id: i64,
    category: &str,
    package: &str,
    pkg_path: &str,
) -> Result<(i64, i64), sqlx::Error> {
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
    .execute(&mut **tx)
    .await?;
    let (cat_id,): (i64,) =
        sqlx::query_as("SELECT id FROM categories WHERE library_id=? AND name=?")
            .bind(lib_id)
            .bind(category)
            .fetch_one(&mut **tx)
            .await?;
    sqlx::query(
        "INSERT INTO packages(category_id,name,path) VALUES(?,?,?)
         ON CONFLICT(category_id,name) DO UPDATE SET path=excluded.path",
    )
    .bind(cat_id)
    .bind(package)
    .bind(pkg_path)
    .execute(&mut **tx)
    .await?;
    let (pkg_id,): (i64,) = sqlx::query_as("SELECT id FROM packages WHERE category_id=? AND name=?")
        .bind(cat_id)
        .bind(package)
        .fetch_one(&mut **tx)
        .await?;
    Ok((cat_id, pkg_id))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// M1 验收测试：扫描真实 GameAssets 库，验证文件数、耗时、未知扩展名。
    /// 跳过条件：库目录不存在时跳过（CI 环境无此目录）。
    #[tokio::test]
    async fn scan_real_gameassets() {
        let root = Path::new("D:\\Xiaoke\\GameAssets");
        if !root.exists() {
            eprintln!("跳过：GameAssets 目录不存在");
            return;
        }
        // 用临时 db，避免污染正式数据
        let tmp = std::env::temp_dir().join(format!("xiaoke_test_{}.db", std::process::id()));
        let _ = std::fs::remove_file(&tmp);
        let opts = sqlx::sqlite::SqliteConnectOptions::new()
            .filename(&tmp)
            .create_if_missing(true);
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(opts)
            .await
            .expect("connect tmp db");
        sqlx::query(include_str!("../migrations/0001_init.sql"))
            .execute(&pool)
            .await
            .expect("migrate");

        let now = chrono::Utc::now().timestamp();
        sqlx::query("INSERT INTO libraries(name,root_path,created_at) VALUES(?,?,?)")
            .bind("test")
            .bind(root.to_string_lossy().to_string())
            .bind(now)
            .execute(&pool)
            .await
            .unwrap();
        let (lib_id,): (i64,) = sqlx::query_as("SELECT id FROM libraries WHERE root_path=?")
            .bind(root.to_string_lossy().to_string())
            .fetch_one(&pool)
            .await
            .unwrap();

        let report = scan_into(&pool, lib_id, root).await.expect("scan");
        println!("ScanReport: {:?}", report);

        // 验收断言
        assert!(
            report.total_files > 40000,
            "文件数应接近 4.2 万，实际 {}",
            report.total_files
        );
        assert!(
            report.duration_ms < 120000,
            "扫描应 < 2 分钟，实际 {} ms",
            report.duration_ms
        );
        // 内置类型表应覆盖库内全部格式：未知扩展名应很少
        println!("未知扩展名: {:?}", report.unknown_extensions);
        // ogg/ttf 不应是 other
        let ogg_kind: Vec<(String,)> =
            sqlx::query_as("SELECT DISTINCT kind FROM files WHERE ext='ogg' AND deleted=0")
                .fetch_all(&pool)
                .await
                .unwrap();
        assert!(
            ogg_kind.iter().all(|(k,)| k == "audio"),
            "ogg 应分类为 audio，实际 {:?}",
            ogg_kind
        );
        let ttf_kind: Vec<(String,)> =
            sqlx::query_as("SELECT DISTINCT kind FROM files WHERE ext='ttf' AND deleted=0")
                .fetch_all(&pool)
                .await
                .unwrap();
        assert!(
            ttf_kind.iter().all(|(k,)| k == "font"),
            "ttf 应分类为 font，实际 {:?}",
            ttf_kind
        );

        // 分类数应为 12（编号 01-13，缺 03，_下载脚本 被跳过）
        let (cat_count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM categories")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(cat_count, 12, "分类数应为 12，实际 {}", cat_count);

        let _ = std::fs::remove_file(&tmp);
    }
}
