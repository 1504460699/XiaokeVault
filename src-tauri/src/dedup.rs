use crate::error::AppError;
use serde::Serialize;
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;
use tauri::State;

#[derive(Debug, Serialize)]
pub struct DupMember {
    pub file_id: Option<i64>,
    pub package_id: Option<i64>,
    pub package_name: Option<String>,
    pub rel_path: String,
    pub role: String,
}

#[derive(Debug, Serialize)]
pub struct DupGroup {
    pub id: i64,
    pub reason: String,
    pub detail: Option<String>,
    pub members: Vec<DupMember>,
}

#[derive(Debug, Serialize)]
pub struct DedupReport {
    pub groups: i64,
    pub removable_files: i64,
    pub removable_bytes: i64,
}

/// 把 zip 名规整成可匹配的形式：%20 / 空格 / x20 / 下划线 统一去掉
fn norm(s: &str) -> String {
    s.to_lowercase()
        .replace("%20", "")
        .replace(" ", "")
        .replace("x20", "")
        .replace("_", "")
        .replace(".zip", "")
        .replace(".7z", "")
        .replace(".rar", "")
}

/// 检测全部重复，写入 duplicate_groups（先清空旧结果）
#[tauri::command]
pub async fn run_dedup(lib_id: i64, pool: State<'_, SqlitePool>) -> Result<DedupReport, AppError> {
    let now = chrono::Utc::now().timestamp();
    sqlx::query("DELETE FROM duplicate_members")
        .execute(&*pool)
        .await
        ?;
    sqlx::query("DELETE FROM duplicate_groups")
        .execute(&*pool)
        .await
        ?;

    let mut groups = 0i64;
    let mut removable_files = 0i64;
    let mut removable_bytes = 0i64;

    // 取所有包的磁盘路径（限当前库）
    let pkgs: Vec<(i64, String)> = sqlx::query_as(
        "SELECT id, path FROM packages
         WHERE category_id IN (SELECT id FROM categories WHERE library_id=?)",
    )
    .bind(lib_id)
    .fetch_all(&*pool)
    .await
    ?;

    for (pkg_id, pkg_path) in &pkgs {
        let pkg_dir = std::path::Path::new(pkg_path);
        if !pkg_dir.is_dir() {
            continue;
        }
        let mut zips: Vec<String> = vec![];
        let mut dirs: Vec<String> = vec![];
        if let Ok(rd) = std::fs::read_dir(pkg_dir) {
            for e in rd.flatten() {
                let name = e.file_name().to_string_lossy().to_string();
                let lower = name.to_lowercase();
                if lower.ends_with(".zip") || lower.ends_with(".7z") || lower.ends_with(".rar") {
                    zips.push(name);
                } else if e.path().is_dir() {
                    dirs.push(name);
                }
            }
        }
        // 配对：zip 名规整后 ≈ 某目录名规整后
        for zip_name in &zips {
            let zn = norm(zip_name);
            if zn.is_empty() {
                continue;
            }
            let mut matched = false;
            for dir_name in &dirs {
                let dn = norm(dir_name);
                if dn.is_empty() {
                    continue;
                }
                if zn == dn || dn.contains(&zn) || zn.contains(&dn) {
                    let (gid,): (i64,) = sqlx::query_as(
                        "INSERT INTO duplicate_groups(reason,detail,created_at) VALUES('zip_extracted',?,?) RETURNING id",
                    )
                    .bind(format!("压缩包 {} 已有解压目录 {}", zip_name, dir_name))
                    .bind(now)
                    .fetch_one(&*pool)
                    .await
                    ?;
                    let zip_file: Option<(i64,)> = sqlx::query_as(
                        "SELECT id FROM files WHERE package_id=? AND rel_path=? AND kind='archive'",
                    )
                    .bind(pkg_id)
                    .bind(zip_name)
                    .fetch_optional(&*pool)
                    .await
                    ?;
                    if let Some((fid,)) = zip_file {
                        sqlx::query(
                            "INSERT INTO duplicate_members(group_id,file_id,package_id,role) VALUES(?,?,?,'remove')",
                        )
                        .bind(gid)
                        .bind(fid)
                        .bind(pkg_id)
                        .execute(&*pool)
                        .await
                        ?;
                        let (bytes,): (i64,) = sqlx::query_as("SELECT bytes FROM files WHERE id=?")
                            .bind(fid)
                            .fetch_one(&*pool)
                            .await
                            ?;
                        removable_files += 1;
                        removable_bytes += bytes;
                    }
                    groups += 1;
                    matched = true;
                    break;
                }
            }
            void(matched);
        }
    }

    // ---- 2. likely_backup：同分类下包名相似的版本/备份 ----
    // 取每个分类下的包，按规整后名字分组，同组内配对
    let cats: Vec<(i64,)> = sqlx::query_as(
        "SELECT id FROM categories WHERE library_id=? ORDER BY id")
        .bind(lib_id).fetch_all(&*pool).await?;
    for (cat_id,) in cats {
        let cat_pkgs: Vec<(i64, String, i64)> = sqlx::query_as(
            "SELECT id, name, file_count FROM packages WHERE category_id=? ORDER BY name")
            .bind(cat_id).fetch_all(&*pool).await?;
        // 预加载已忽略的包对集合
        let dismissed: Vec<(i64, i64)> = sqlx::query_as(
            "SELECT package_a, package_b FROM dismissed_pairs")
            .fetch_all(&*pool).await?;
        let dismissed_set: std::collections::HashSet<(i64, i64)> = dismissed.into_iter().collect();
        for i in 0..cat_pkgs.len() {
            for j in (i + 1)..cat_pkgs.len() {
                let (id_a, name_a, fc_a) = &cat_pkgs[i];
                let (id_b, name_b, fc_b) = &cat_pkgs[j];
                // 跳过已忽略的包对
                let key = if id_a < id_b { (*id_a, *id_b) } else { (*id_b, *id_a) };
                if dismissed_set.contains(&key) { continue; }
                let na = backup_norm(name_a);
                let nb = backup_norm(name_b);
                if na.len() < 4 || nb.len() < 4 { continue; }
                let sim = common_prefix_ratio(&na, &nb);
                // 必须同时满足：名字相似度高 + 至少一方含明确的"备份/版本"语义信号
                // （避免同资源不同分辨率被误报，如 32x32 vs 64x64）
                let looks_like_backup = sim >= 0.7 && has_backup_signal(name_a, name_b);
                if !looks_like_backup { continue; }
                let max_fc = (*fc_a).max(*fc_b) as f64;
                if max_fc > 0.0 {
                    let diff = ((*fc_a - *fc_b).abs() as f64) / max_fc;
                    if diff <= 0.5 || *fc_a == 0 || *fc_b == 0 {
                        let (gid,): (i64,) = sqlx::query_as(
                            "INSERT INTO duplicate_groups(reason,detail,created_at) VALUES('likely_backup',?,?) RETURNING id")
                            .bind(format!("疑似备份：「{}」({}文件) 与 「{}」({}文件) 名称相似且含版本/备份标识，请人工确认", name_a, fc_a, name_b, fc_b))
                            .bind(now)
                            .fetch_one(&*pool).await?;
                        sqlx::query("INSERT OR IGNORE INTO duplicate_members(group_id,package_id,role) VALUES(?,?,'keep')")
                            .bind(gid).bind(id_a).execute(&*pool).await?;
                        sqlx::query("INSERT OR IGNORE INTO duplicate_members(group_id,package_id,role) VALUES(?,?,'keep')")
                            .bind(gid).bind(id_b).execute(&*pool).await?;
                        groups += 1;
                    }
                }
            }
        }
    }

    // hash 检测暂不启用（4.2万文件算 sha256 成本高，likely_backup 已覆盖主要场景）

    Ok(DedupReport {
        groups,
        removable_files,
        removable_bytes,
    })
}

#[inline]
fn void(_: bool) {}

/// 判断是否含明确的"备份/版本"语义信号（避免分辨率/规格差异被误报）
/// 至少一方含这些词，才认为是可能的备份
fn has_backup_signal(a: &str, b: &str) -> bool {
    let signals = [
        "完整版", "老版", "新版", "备份", "backup", "copy", "_full",
        "_supplemental", "v2", "v3", "2010", "2017", "2018", "2019", "2020",
        "图集散图",  // 散图 vs 整理版
    ];
    let combined = format!("{} {}", a.to_lowercase(), b.to_lowercase());
    signals.iter().any(|s| combined.contains(s))
}

/// 包名规整：去掉版本/年份/后缀词，小写，便于相似度比对
fn backup_norm(s: &str) -> String {
    s.to_lowercase()
        .replace("完整版", "")
        .replace("老版", "")
        .replace("新版", "")
        .replace("_full", "")
        .replace("_supplemental", "")
        .replace("_individual_organized_tiles_sprites", "")
        .replace("图集散图", "")
        .replace("散图", "")
        .replace(|c: char| c.is_ascii_digit(), "")  // 去数字（年份/版本号）
        .replace("_", "")
        .replace(" ", "")
}

/// 两个字符串的共同前缀占较短者的比例
fn common_prefix_ratio(a: &str, b: &str) -> f64 {
    let min_len = a.len().min(b.len());
    if min_len == 0 { return 0.0; }
    let mut common = 0;
    for (ca, cb) in a.chars().zip(b.chars()) {
        if ca == cb { common += 1; } else { break; }
    }
    common as f64 / min_len as f64
}

/// 关闭/忽略一个重复组（人工已确认，记录包对，删除该组记录，不再提醒）
#[tauri::command]
pub async fn dismiss_duplicate_group(
    group_id: i64,
    pool: State<'_, SqlitePool>,
) -> Result<(), AppError> {
    let now = chrono::Utc::now().timestamp();
    // 取该组的所有 package_id 成员，两两记入 dismissed_pairs
    let pks: Vec<i64> = sqlx::query_scalar(
        "SELECT DISTINCT package_id FROM duplicate_members WHERE group_id=? AND package_id IS NOT NULL",
    )
    .bind(group_id)
    .fetch_all(&*pool)
    .await
    ?;
    for i in 0..pks.len() {
        for j in (i + 1)..pks.len() {
            let (a, b) = if pks[i] < pks[j] { (pks[i], pks[j]) } else { (pks[j], pks[i]) };
            sqlx::query("INSERT OR IGNORE INTO dismissed_pairs(package_a,package_b,created_at) VALUES(?,?,?)")
                .bind(a).bind(b).bind(now).execute(&*pool).await
                ?;
        }
    }
    sqlx::query("DELETE FROM duplicate_members WHERE group_id=?")
        .bind(group_id)
        .execute(&*pool)
        .await
        ?;
    sqlx::query("DELETE FROM duplicate_groups WHERE id=?")
        .bind(group_id)
        .execute(&*pool)
        .await
        ?;
    Ok(())
}

/// 取所有重复组
#[tauri::command]
pub async fn get_duplicate_groups(pool: State<'_, SqlitePool>) -> Result<Vec<DupGroup>, AppError> {
    let groups: Vec<(i64, String, Option<String>)> =
        sqlx::query_as("SELECT id, reason, detail FROM duplicate_groups ORDER BY id")
            .fetch_all(&*pool)
            .await
            ?;
    let mut out = Vec::new();
    for (gid, reason, detail) in groups {
        let members: Vec<(Option<i64>, Option<i64>, Option<String>, String, String)> =
            sqlx::query_as(
                "SELECT dm.file_id, dm.package_id, p.name, COALESCE(f.rel_path,''), dm.role
                 FROM duplicate_members dm
                 LEFT JOIN files f ON f.id=dm.file_id
                 LEFT JOIN packages p ON p.id=dm.package_id
                 WHERE dm.group_id=?",
            )
            .bind(gid)
            .fetch_all(&*pool)
            .await
            ?;
        out.push(DupGroup {
            id: gid,
            reason,
            detail,
            members: members
                .into_iter()
                .map(|(fid, pid, pn, rp, role)| DupMember {
                    file_id: fid,
                    package_id: pid,
                    package_name: pn,
                    rel_path: rp,
                    role,
                })
                .collect(),
        });
    }
    Ok(out)
}

/// 把单个文件移到备份目录，返回目标路径。
/// backup_root: 用户指定的备份根目录；为空则用 app data/trash。
fn move_to_backup(src: &std::path::Path, abs_pkg: &str, rel: &str, backup_root: &str) -> Result<std::path::PathBuf, AppError> {
    let mut base = if backup_root.is_empty() {
        let mut t = crate::db::data_root();
        t.push("trash");
        t
    } else {
        std::path::PathBuf::from(backup_root)
    };
    base.push(abs_pkg.replace(':', "_"));
    base.push(rel);
    if let Some(parent) = base.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::rename(src, &base)?;
    Ok(base)
}

/// 软删除单个冗余文件
#[tauri::command]
pub async fn remove_duplicate(
    file_id: i64,
    backup_root: String,
    pool: State<'_, SqlitePool>,
) -> Result<String, AppError> {
    let (abs_pkg, rel): (String, String) = sqlx::query_as(
        "SELECT p.path, f.rel_path FROM files f JOIN packages p ON p.id=f.package_id WHERE f.id=?",
    )
    .bind(file_id)
    .fetch_one(&*pool)
    .await
    ?;
    let src = std::path::Path::new(&abs_pkg).join(&rel);
    let msg = if src.exists() {
        let dest = move_to_backup(&src, &abs_pkg, &rel, &backup_root)?;
        sqlx::query("UPDATE files SET deleted=1 WHERE id=?")
            .bind(file_id)
            .execute(&*pool)
            .await
            ?;
        dest.to_string_lossy().to_string()
    } else {
        sqlx::query("UPDATE files SET deleted=1 WHERE id=?")
            .bind(file_id)
            .execute(&*pool)
            .await
            ?;
        "文件已不存在，仅标记删除".into()
    };
    Ok(msg)
}

/// 一键去重：批量移除所有 role=remove 的成员到备份目录
#[derive(Debug, Serialize)]
pub struct BatchRemoveResult {
    pub removed: i64,
    pub failed: i64,
}

#[tauri::command]
pub async fn remove_all_duplicates(
    backup_root: String,
    pool: State<'_, SqlitePool>,
) -> Result<BatchRemoveResult, AppError> {
    let file_ids: Vec<i64> = sqlx::query_scalar(
        "SELECT file_id FROM duplicate_members WHERE role='remove' AND file_id IS NOT NULL",
    )
    .fetch_all(&*pool)
    .await
    ?;
    let mut removed = 0i64;
    let mut failed = 0i64;
    for fid in file_ids {
        let (abs_pkg, rel): (String, String) = sqlx::query_as(
            "SELECT p.path, f.rel_path FROM files f JOIN packages p ON p.id=f.package_id WHERE f.id=?",
        )
        .bind(fid)
        .fetch_one(&*pool)
        .await
        ?;
        let src = std::path::Path::new(&abs_pkg).join(&rel);
        let ok = if src.exists() {
            move_to_backup(&src, &abs_pkg, &rel, &backup_root).is_ok()
        } else {
            true
        };
        if ok {
            let _ = sqlx::query("UPDATE files SET deleted=1 WHERE id=?")
                .bind(fid)
                .execute(&*pool)
                .await;
            removed += 1;
        } else {
            failed += 1;
        }
    }
    // 清空已处理的重复组
    sqlx::query("DELETE FROM duplicate_members")
        .execute(&*pool).await?;
    sqlx::query("DELETE FROM duplicate_groups")
        .execute(&*pool).await?;
    Ok(BatchRemoveResult { removed, failed })
}

/// 计算文件 sha256（hash 检测备用，M5 v1 暂不主动调用）
#[allow(dead_code)]
fn file_hash(path: &std::path::Path) -> Option<String> {
    let mut hasher = Sha256::new();
    let mut f = std::fs::File::open(path).ok()?;
    std::io::copy(&mut f, &mut hasher).ok()?;
    Some(format!("{:x}", hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// norm：去空格/下划线/扩展名/小写
    #[test]
    fn test_norm_basic() {
        assert_eq!(norm("My_Asset.zip"), "myasset");
        assert_eq!(norm("Hero%20Pack.7z"), "heropack");
        assert_eq!(norm("A B C"), "abc");
    }

    #[test]
    fn test_norm_lowercase_and_extension() {
        // 扩展名应被去掉
        assert!(!norm("file.ZIP").contains("zip"));
        // 大写转小写
        assert_eq!(norm("ABC"), "abc");
    }

    /// has_backup_signal：含版本/年份/备份词才返回 true
    #[test]
    fn test_has_backup_signal_year() {
        assert!(has_backup_signal("Trees2010", "Trees"));
        assert!(has_backup_signal("Pack", "Pack v2"));
    }

    #[test]
    fn test_has_backup_signal_words() {
        assert!(has_backup_signal("完整版资源", "资源"));
        assert!(has_backup_signal("a", "b copy"));
        assert!(has_backup_signal("Dungeon_full", "Dungeon"));
    }

    #[test]
    fn test_has_backup_signal_none() {
        // 仅分辨率/规格差异，不含信号词 → false（避免误报）
        assert!(!has_backup_signal("Hero_64px", "Hero_128px"));
        assert!(!has_backup_signal("tree", "bush"));
    }

    /// backup_norm：去版本/年份/后缀词/数字/下划线
    #[test]
    fn test_backup_norm_strips_modifiers() {
        let n = backup_norm("Trees_2010_完整版");
        assert!(!n.contains("完整版"));
        assert!(!n.contains("2010"));
        assert!(!n.contains("_"));
        assert!(n.contains("trees"));
    }

    #[test]
    fn test_backup_norm_digits_removed() {
        // 数字（年份/版本号）应被去掉
        assert!(!backup_norm("Pack2020").contains("2020"));
        assert!(!backup_norm("v3final").contains('3'));
    }

    /// common_prefix_ratio：共同前缀占较短者比例
    #[test]
    fn test_common_prefix_ratio_full() {
        assert!((common_prefix_ratio("abc", "abcde") - 1.0).abs() < 1e-9);
    }

    #[test]
    fn test_common_prefix_ratio_partial() {
        // "tree" vs "treesong"，共同前缀 4，较短者 4 → 1.0
        assert!((common_prefix_ratio("tree", "treesong") - 1.0).abs() < 1e-9);
        // "abc" vs "axc"，共同前缀 1（a），较短者 3 → 1/3
        let r = common_prefix_ratio("abc", "axc");
        assert!((r - 1.0 / 3.0).abs() < 1e-9);
    }

    #[test]
    fn test_common_prefix_ratio_empty() {
        assert_eq!(common_prefix_ratio("", "abc"), 0.0);
        assert_eq!(common_prefix_ratio("abc", ""), 0.0);
    }
}

