use crate::error::AppError;
use serde::Serialize;
use sqlx::SqlitePool;
use std::collections::BTreeSet;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter, State};
use zip::write::SimpleFileOptions;

#[derive(Debug, Serialize, Clone)]
pub struct ExportProgress {
    pub stage: String,
    pub done: u64,
    pub total: u64,
    pub current: String,
}

#[derive(Debug, Serialize)]
pub struct ExportResult {
    pub output_path: String,
    pub file_count: u64,
    pub total_bytes: u64,
}

struct ExportItem {
    src: PathBuf,
    dest_rel: String,
    category: String,
    package: String,
    name: String,
    ext: String,
    kind: String,
    bytes: i64,
}

async fn resolve_export_items(pool: &SqlitePool, project_id: i64) -> Result<Vec<ExportItem>, AppError> {
    let rows: Vec<(String, String, String, String, String, String, i64, String)> =
        sqlx::query_as(
            "SELECT p.path, c.name, p.name, f.rel_path, f.name, f.ext, f.bytes, f.kind
             FROM files f
             JOIN packages p ON p.id=f.package_id
             JOIN categories c ON c.id=p.category_id
             WHERE f.deleted=0 AND (
               f.package_id IN (SELECT package_id FROM selections WHERE project_id=? AND scope='package')
               OR f.id IN (SELECT file_id FROM selections WHERE project_id=? AND scope='file')
             ) AND f.id NOT IN (SELECT file_id FROM selections WHERE project_id=? AND scope='exclude')",
        )
        .bind(project_id)
        .bind(project_id)
        .bind(project_id)
        .fetch_all(pool)
        .await
        ?;

    let mut items = Vec::new();
    for (pkg_path, category, package, rel, name, ext, bytes, kind) in rows {
        let src = PathBuf::from(&pkg_path).join(&rel);
        let dest_rel = format!("assets/{}/{}/{}", category, package, rel.replace('\\', "/"));
        items.push(ExportItem {
            src,
            dest_rel,
            category,
            package,
            name,
            ext,
            kind,
            bytes,
        });
    }
    Ok(items)
}

#[tauri::command]
pub async fn run_export(
    app: AppHandle,
    project_id: i64,
    format: String,
    export_root: String,
    pool: State<'_, SqlitePool>,
) -> Result<ExportResult, AppError> {
    let (proj_name,): (String,) = sqlx::query_as("SELECT name FROM projects WHERE id=?")
        .bind(project_id)
        .fetch_one(&*pool)
        .await
        ?;

    // 更新项目的 export_root（用户在对话框选择的导出位置）
    sqlx::query("UPDATE projects SET export_root=? WHERE id=?")
        .bind(&export_root)
        .bind(project_id)
        .execute(&*pool)
        .await
        ?;

    let items = resolve_export_items(&pool, project_id).await?;
    let total = items.len() as u64;
    let out_root = PathBuf::from(&export_root);
    fs::create_dir_all(&out_root)?;

    let emit = |app: &AppHandle, stage: &str, done: u64, total: u64, current: &str| {
        let _ = app.emit(
            "export://progress",
            ExportProgress {
                stage: stage.to_string(),
                done,
                total,
                current: current.to_string(),
            },
        );
    };

    let result = if format == "zip" {
        let zip_path = out_root.join(format!("{}.zip", sanitize(&proj_name)));
        let file = fs::File::create(&zip_path)?;
        let mut writer = zip::ZipWriter::new(file);
        let opts = SimpleFileOptions::default();
        let mut done = 0u64;
        for it in &items {
            emit(&app, "copy", done, total, &it.dest_rel);
            if let Ok(mut f) = fs::File::open(&it.src) {
                if writer.start_file(&it.dest_rel, opts).is_ok() {
                    let _ = std::io::copy(&mut f, &mut writer);
                }
            }
            done += 1;
        }
        write_credits_to_zip(&mut writer, &pool, &items).await?;
        write_manifest_to_zip(&mut writer, &proj_name, "zip", &items)?;
        writer.finish()?;
        emit(&app, "done", done, total, "");
        ExportResult {
            output_path: zip_path.to_string_lossy().to_string(),
            file_count: total,
            total_bytes: items.iter().map(|i| i.bytes as u64).sum(),
        }
    } else {
        let proj_dir = out_root.join(sanitize(&proj_name));
        fs::create_dir_all(&proj_dir)?;
        let mut done = 0u64;
        for it in &items {
            emit(&app, "copy", done, total, &it.dest_rel);
            let dest = proj_dir.join(&it.dest_rel);
            if let Some(parent) = dest.parent() {
                let _ = fs::create_dir_all(parent);
            }
            let _ = fs::copy(&it.src, &dest);
            done += 1;
        }
        write_credits(&proj_dir, &pool, &items).await?;
        write_manifest(&proj_dir, &proj_name, "folder", &items)?;
        emit(&app, "done", done, total, "");
        ExportResult {
            output_path: proj_dir.to_string_lossy().to_string(),
            file_count: total,
            total_bytes: items.iter().map(|i| i.bytes as u64).sum(),
        }
    };

    Ok(result)
}

fn sanitize(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            _ => c,
        })
        .collect()
}

async fn build_credits(
    pool: &SqlitePool,
    items: &[ExportItem],
) -> Result<(String, serde_json::Value), AppError> {
    let mut pkg_set: BTreeSet<String> = BTreeSet::new();
    for it in items {
        pkg_set.insert(it.package.clone());
    }
    let mut lines: Vec<String> = vec!["# CREDITS".to_string()];
    let mut json_arr: Vec<serde_json::Value> = Vec::new();
    for pkg_name in &pkg_set {
        let row: Option<(Option<String>, Option<String>, Option<String>)> = sqlx::query_as(
            "SELECT source_url, source_title, license FROM packages WHERE name=? LIMIT 1",
        )
        .bind(pkg_name)
        .fetch_optional(pool)
        .await
        ?;
        let (url, title, license) = row.unwrap_or((None, None, None));
        let display_title = title.clone().unwrap_or_else(|| pkg_name.clone());
        lines.push(format!(
            "- {} [{}] {}",
            display_title,
            license.as_deref().unwrap_or("UNKNOWN"),
            url.as_deref().unwrap_or(""),
        ));
        json_arr.push(serde_json::json!({
            "package": pkg_name,
            "title": display_title,
            "license": license,
            "source_url": url,
        }));
    }
    Ok((lines.join("\n"), serde_json::json!({ "credits": json_arr })))
}

async fn write_credits(proj_dir: &Path, pool: &SqlitePool, items: &[ExportItem]) -> Result<(), AppError> {
    let (txt, json) = build_credits(pool, items).await?;
    fs::write(proj_dir.join("CREDITS.txt"), txt)?;
    fs::write(
        proj_dir.join("CREDITS.json"),
        serde_json::to_string_pretty(&json).unwrap(),
    )
    ?;
    Ok(())
}

fn write_manifest(proj_dir: &Path, project_name: &str, format: &str, items: &[ExportItem]) -> Result<(), AppError> {
    let manifest = serde_json::json!({
        "project": project_name,
        "exported_at": chrono::Utc::now().timestamp(),
        "format": format,
        "total_files": items.len(),
        "total_bytes": items.iter().map(|i| i.bytes as u64).sum::<u64>(),
        "files": items.iter().map(|i| serde_json::json!({
            "export_path": i.dest_rel,
            "category": i.category,
            "package": i.package,
            "source_path": i.src.to_string_lossy().replace("\\", "/"),
            "name": i.name,
            "ext": i.ext,
            "kind": i.kind,
            "bytes": i.bytes,
        })).collect::<Vec<_>>(),
    });
    fs::write(
        proj_dir.join("manifest.json"),
        serde_json::to_string_pretty(&manifest).unwrap(),
    )
    ?;
    Ok(())
}

async fn write_credits_to_zip(
    writer: &mut zip::ZipWriter<fs::File>,
    pool: &SqlitePool,
    items: &[ExportItem],
) -> Result<(), AppError> {
    let (txt, json) = build_credits(pool, items).await?;
    let opts = SimpleFileOptions::default();
    writer.start_file("CREDITS.txt", opts)?;
    writer.write_all(txt.as_bytes())?;
    writer.start_file("CREDITS.json", opts)?;
    writer
        .write_all(serde_json::to_string_pretty(&json).unwrap().as_bytes())
        ?;
    Ok(())
}

fn write_manifest_to_zip(
    writer: &mut zip::ZipWriter<fs::File>,
    project_name: &str,
    format: &str,
    items: &[ExportItem],
) -> Result<(), AppError> {
    let manifest = serde_json::json!({
        "project": project_name,
        "exported_at": chrono::Utc::now().timestamp(),
        "format": format,
        "total_files": items.len(),
        "total_bytes": items.iter().map(|i| i.bytes as u64).sum::<u64>(),
        "files": items.iter().map(|i| serde_json::json!({
            "export_path": i.dest_rel,
            "category": i.category,
            "package": i.package,
            "source_path": i.src.to_string_lossy().replace("\\", "/"),
            "name": i.name,
            "ext": i.ext,
            "kind": i.kind,
            "bytes": i.bytes,
        })).collect::<Vec<_>>(),
    });
    let opts = SimpleFileOptions::default();
    writer.start_file("manifest.json", opts)?;
    writer
        .write_all(serde_json::to_string_pretty(&manifest).unwrap().as_bytes())
        ?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// sanitize：Windows 非法路径字符替换为下划线
    #[test]
    fn test_sanitize_replaces_illegal_chars() {
        assert_eq!(sanitize("a/b\\c"), "a_b_c");
        assert_eq!(sanitize("file:name"), "file_name");
        assert_eq!(sanitize("a*b?c"), "a_b_c");
        assert_eq!(sanitize("a<b>c|d"), "a_b_c_d");
        assert_eq!(sanitize("say\"hi"), "say_hi");
    }

    #[test]
    fn test_sanitize_keeps_safe_chars() {
        // 普通字符、中文、数字、点、空格应保留
        assert_eq!(sanitize("素材包"), "素材包");
        assert_eq!(sanitize("file_1.txt"), "file_1.txt");
        assert_eq!(sanitize("Hero Pack 2"), "Hero Pack 2");
    }

    #[test]
    fn test_sanitize_empty_and_complex() {
        assert_eq!(sanitize(""), "");
        // 混合：保留合法、替换非法
        assert_eq!(sanitize("树/叶:2"), "树_叶_2");
    }
}

