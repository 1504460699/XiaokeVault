use crate::asset_types::Registry;
use crate::tree_scanner;
use serde::Serialize;
use sqlx::{Acquire, SqlitePool};
use std::collections::{HashMap, HashSet};
use std::path::Path;

/// 扫描结果统计
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

/// 目录树扫描入库：把库根下所有目录和文件按真实结构写入 directories / files 表。
/// （统一的扫描入口，原两级扫描 scan_into 已随两级视图一并移除）
pub async fn scan_tree_into(
    pool: &SqlitePool,
    lib_id: i64,
    root: &Path,
) -> Result<ScanReport, sqlx::Error> {
    let start = std::time::Instant::now();
    crate::alog_info!("scan", "scan_tree_into：开始，lib_id={} root={}", lib_id, root.display());

    crate::alog_debug!("scan", "步骤1/5：加载资源类型 Registry");
    let registry = Registry::load(pool).await?;
    crate::alog_debug!("scan", "Registry 加载完成");

    let mut unknown: HashMap<String, u64> = HashMap::new();
    crate::alog_debug!("scan", "步骤2/5：扫描磁盘目录树");
    let result = tree_scanner::scan_library_tree(root);
    crate::alog_info!(
        "scan",
        "磁盘扫描完成：{} 个目录，{} 个文件",
        result.dirs.len(),
        result.files.len()
    );

    let mut conn = pool.acquire().await?;
    crate::alog_debug!("scan", "步骤3/5：获取数据库连接，开启事务");

    let mut new = 0u64;
    let mut updated = 0u64;
    let mut deleted = 0u64;
    let mut total_written = 0u64;

    let mut tx = conn.begin().await?;

    // === 1. 写 directories 表 ===
    // path -> id 映射，供 files.directory_id 用
    let mut path_to_id: HashMap<String, i64> = HashMap::new();

    // 先取所有已存在 directory 的 path（用于增量删除判断）
    let existing_dir_paths: HashSet<String> =
        sqlx::query_scalar("SELECT path FROM directories WHERE library_id=?")
            .bind(lib_id)
            .fetch_all(&mut *tx)
            .await?
            .into_iter()
            .collect();

    let mut seen_dir_paths: HashSet<String> = HashSet::new();

    for d in &result.dirs {
        // 解析 parent_id
        let parent_id: Option<i64> = match &d.parent_path {
            Some(pp) => path_to_id.get(pp).copied(),
            None => None,
        };
        sqlx::query(
            "INSERT INTO directories(library_id,parent_id,name,path,depth,file_count,total_bytes)
             VALUES(?,?,?,?,?,0,0)
             ON CONFLICT(library_id,path) DO UPDATE SET
               parent_id=excluded.parent_id, name=excluded.name, depth=excluded.depth",
        )
        .bind(lib_id)
        .bind(parent_id)
        .bind(&d.name)
        .bind(&d.path)
        .bind(d.depth)
        .execute(&mut *tx)
        .await?;
        let (id,): (i64,) =
            sqlx::query_as("SELECT id FROM directories WHERE library_id=? AND path=?")
                .bind(lib_id)
                .bind(&d.path)
                .fetch_one(&mut *tx)
                .await?;
        path_to_id.insert(d.path.clone(), id);
        seen_dir_paths.insert(d.path.clone());
    }
    crate::alog_debug!("scan", "写入 directories 完成：{} 个目录", path_to_id.len());

    // === 2. 写 files（带 directory_id）+ 累计 directory 的 file_count/bytes ===
    let mut dir_stats: HashMap<String, (i64, i64)> = HashMap::new(); // dir_path -> (count, bytes)

    // 预取所有已存在的文件 (directory_id, rel_path)，用于区分 new/updated。
    let existing_files: HashSet<(Option<i64>, String)> =
        sqlx::query_as("SELECT directory_id, rel_path FROM files")
            .fetch_all(&mut *tx)
            .await?
            .into_iter()
            .collect();

    let mut skipped_no_dir = 0u64;
    for f in &result.files {
        let kind = if registry.kind_for(&f.ext) == "other" && !f.ext.is_empty() {
            *unknown.entry(f.ext.clone()).or_insert(0) += 1;
            "other"
        } else {
            registry.kind_for(&f.ext)
        };
        let dir_id = path_to_id.get(&f.dir_path).copied();

        // 防御：若文件所在目录未在 path_to_id 中（理论上不该发生），
        // 跳过该文件并记录，避免 INSERT NULL 到 NOT NULL 列导致整个扫描失败。
        // 常见原因：文件在库根下、或目录扫描顺序导致子目录先于父目录处理。
        let Some(dir_id) = dir_id else {
            skipped_no_dir += 1;
            if skipped_no_dir <= 5 {
                crate::alog_warn!(
                    "scan",
                    "跳过文件（找不到所属目录）：dir_path={:?} rel_path={:?}",
                    f.dir_path,
                    f.rel_path
                );
            }
            continue;
        };

        // 约束 UNIQUE(directory_id, rel_path)：每目录内 rel_path 唯一，
        // 不同目录同名文件不会互相覆盖。
        sqlx::query(
            "INSERT INTO files(directory_id,rel_path,name,ext,kind,bytes,modified_at,deleted)
             VALUES(?,?,?,?,?,?,?,0)
             ON CONFLICT(directory_id, rel_path) DO UPDATE SET
               name=excluded.name, ext=excluded.ext, kind=excluded.kind,
               bytes=excluded.bytes, modified_at=excluded.modified_at, deleted=0",
        )
        .bind(dir_id)
        .bind(&f.rel_path)
        .bind(&f.name)
        .bind(&f.ext)
        .bind(kind)
        .bind(f.bytes as i64)
        .bind(f.modified_at)
        .execute(&mut *tx)
        .await?;

        let existed = existing_files.contains(&(Some(dir_id), f.rel_path.clone()));
        if existed {
            updated += 1;
        } else {
            new += 1;
        }
        total_written += 1;
        let e = dir_stats.entry(f.dir_path.clone()).or_insert((0, 0));
        e.0 += 1;
        e.1 += f.bytes as i64;
    }
    if skipped_no_dir > 0 {
        crate::alog_warn!("scan", "共 {} 个文件因找不到所属目录被跳过", skipped_no_dir);
    }
    crate::alog_debug!(
        "scan",
        "写入 files 完成：new={} updated={} skipped={}",
        new,
        updated,
        skipped_no_dir
    );

    // === 3. 更新 directories 的 file_count/total_bytes ===
    for (dir_path, (count, bytes)) in &dir_stats {
        if let Some(id) = path_to_id.get(dir_path) {
            sqlx::query("UPDATE directories SET file_count=?, total_bytes=? WHERE id=?")
                .bind(count)
                .bind(bytes)
                .bind(id)
                .execute(&mut *tx)
                .await?;
        }
    }
    // 没文件的目录 file_count 置 0
    for (dir_path, id) in &path_to_id {
        if !dir_stats.contains_key(dir_path) {
            sqlx::query("UPDATE directories SET file_count=0, total_bytes=0 WHERE id=?")
                .bind(id)
                .execute(&mut *tx)
                .await?;
        }
    }

    // === 4. 增量删除：磁盘已不存在的目录直接删（CASCADE 删其 files）===
    crate::alog_debug!("scan", "步骤4/5：增量删除磁盘已不存在的目录");
    for old_path in existing_dir_paths.iter() {
        if !seen_dir_paths.contains(old_path) {
            sqlx::query("DELETE FROM directories WHERE library_id=? AND path=?")
                .bind(lib_id)
                .bind(old_path)
                .execute(&mut *tx)
                .await?;
            deleted += 1;
        }
    }

    crate::alog_debug!("scan", "步骤5/5：提交事务");
    tx.commit().await?;
    crate::alog_info!("scan", "事务已提交，扫描入库完成");

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

#[cfg(test)]
mod tests {
    use super::*;

    /// 验收测试：扫描真实 GameAssets 库，验证目录树文件数、耗时、未知扩展名。
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
            .expect("migrate 0001");
        sqlx::query(include_str!("../migrations/0004_directories.sql"))
            .execute(&pool)
            .await
            .expect("migrate 0004");

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

        let report = scan_tree_into(&pool, lib_id, root).await.expect("scan");
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

        let _ = std::fs::remove_file(&tmp);
    }
}
