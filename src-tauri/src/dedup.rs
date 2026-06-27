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
pub async fn run_dedup(lib_id: i64, pool: State<'_, SqlitePool>) -> Result<DedupReport, String> {
    let now = chrono::Utc::now().timestamp();
    sqlx::query("DELETE FROM duplicate_members")
        .execute(&*pool)
        .await
        .map_err(|e| e.to_string())?;
    sqlx::query("DELETE FROM duplicate_groups")
        .execute(&*pool)
        .await
        .map_err(|e| e.to_string())?;

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
    .map_err(|e| e.to_string())?;

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
                    .map_err(|e| e.to_string())?;
                    let zip_file: Option<(i64,)> = sqlx::query_as(
                        "SELECT id FROM files WHERE package_id=? AND rel_path=? AND kind='archive'",
                    )
                    .bind(pkg_id)
                    .bind(zip_name)
                    .fetch_optional(&*pool)
                    .await
                    .map_err(|e| e.to_string())?;
                    if let Some((fid,)) = zip_file {
                        sqlx::query(
                            "INSERT INTO duplicate_members(group_id,file_id,package_id,role) VALUES(?,?,?,'remove')",
                        )
                        .bind(gid)
                        .bind(fid)
                        .bind(pkg_id)
                        .execute(&*pool)
                        .await
                        .map_err(|e| e.to_string())?;
                        let (bytes,): (i64,) = sqlx::query_as("SELECT bytes FROM files WHERE id=?")
                            .bind(fid)
                            .fetch_one(&*pool)
                            .await
                            .map_err(|e| e.to_string())?;
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

    Ok(DedupReport {
        groups,
        removable_files,
        removable_bytes,
    })
}

#[inline]
fn void(_: bool) {}

/// 取所有重复组
#[tauri::command]
pub async fn get_duplicate_groups(pool: State<'_, SqlitePool>) -> Result<Vec<DupGroup>, String> {
    let groups: Vec<(i64, String, Option<String>)> =
        sqlx::query_as("SELECT id, reason, detail FROM duplicate_groups ORDER BY id")
            .fetch_all(&*pool)
            .await
            .map_err(|e| e.to_string())?;
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
            .map_err(|e| e.to_string())?;
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

/// 软删除：把 file 物理移到 trash，files 标 deleted=1
#[tauri::command]
pub async fn remove_duplicate(file_id: i64, pool: State<'_, SqlitePool>) -> Result<String, String> {
    let (abs_pkg, rel): (String, String) = sqlx::query_as(
        "SELECT p.path, f.rel_path FROM files f JOIN packages p ON p.id=f.package_id WHERE f.id=?",
    )
    .bind(file_id)
    .fetch_one(&*pool)
    .await
    .map_err(|e| e.to_string())?;
    let src = std::path::Path::new(&abs_pkg).join(&rel);
    if src.exists() {
        let mut trash = dirs::data_dir().ok_or("no data dir")?;
        trash.push("com.xiaoke.tauri-app");
        trash.push("trash");
        let ts = chrono::Utc::now().timestamp();
        let dest = trash
            .join(ts.to_string())
            .join(abs_pkg.replace(':', "_"))
            .join(&rel);
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        std::fs::rename(&src, &dest).map_err(|e| e.to_string())?;
        sqlx::query("UPDATE files SET deleted=1 WHERE id=?")
            .bind(file_id)
            .execute(&*pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(dest.to_string_lossy().to_string())
    } else {
        sqlx::query("UPDATE files SET deleted=1 WHERE id=?")
            .bind(file_id)
            .execute(&*pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok("文件已不存在，仅标记删除".into())
    }
}

/// 计算文件 sha256（hash 检测备用，M5 v1 暂不主动调用）
#[allow(dead_code)]
fn file_hash(path: &std::path::Path) -> Option<String> {
    let mut hasher = Sha256::new();
    let mut f = std::fs::File::open(path).ok()?;
    std::io::copy(&mut f, &mut hasher).ok()?;
    Some(format!("{:x}", hasher.finalize()))
}
