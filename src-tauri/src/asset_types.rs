use crate::error::AppError;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::collections::HashMap;

/// 类型注册表条目。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetType {
    pub kind: String,
    pub label: String,
    pub extensions: Vec<String>,
    pub viewer: String,
    pub icon: Option<String>,
    pub is_source: bool,
}

/// 内置默认类型表（设计文档 §3.2）。
/// 编译进二进制，读取时与数据库覆盖项合并。
pub fn builtin_types() -> Vec<AssetType> {
    vec![
        AssetType {
            kind: "image".into(),
            label: "图片".into(),
            extensions: str_vec(["png", "jpg", "jpeg", "webp", "bmp", "tif", "tiff"]),
            viewer: "image".into(),
            icon: Some("image".into()),
            is_source: false,
        },
        AssetType {
            kind: "animated".into(),
            label: "动画".into(),
            extensions: str_vec(["gif"]),
            viewer: "animated".into(),
            icon: Some("animated".into()),
            is_source: false,
        },
        AssetType {
            kind: "vector".into(),
            label: "矢量".into(),
            extensions: str_vec(["svg"]),
            viewer: "vector".into(),
            icon: Some("vector".into()),
            is_source: false,
        },
        AssetType {
            kind: "audio".into(),
            label: "音频".into(),
            extensions: str_vec(["ogg", "mp3", "wav", "flac"]),
            viewer: "audio".into(),
            icon: Some("audio".into()),
            is_source: false,
        },
        AssetType {
            kind: "font".into(),
            label: "字体".into(),
            extensions: str_vec(["ttf", "otf", "eot", "woff", "woff2"]),
            viewer: "font".into(),
            icon: Some("font".into()),
            is_source: false,
        },
        AssetType {
            kind: "text".into(),
            label: "文本数据".into(),
            extensions: str_vec(["txt", "xml", "json", "cs", "sh", "mat", "tmx", "html", "css", "js", "md"]),
            viewer: "text".into(),
            icon: Some("text".into()),
            is_source: false,
        },
        AssetType {
            kind: "model3d".into(),
            label: "3D模型".into(),
            extensions: str_vec(["obj", "mtl", "fbx", "gltf", "glb", "dae", "dds", "tga"]),
            viewer: "3d".into(),
            icon: Some("model3d".into()),
            is_source: false,
        },
        AssetType {
            kind: "source_blend".into(),
            label: "Blender源".into(),
            extensions: str_vec(["blend"]),
            viewer: "binary-source".into(),
            icon: Some("blend".into()),
            is_source: true,
        },
        AssetType {
            kind: "source_pixel".into(),
            label: "像素源".into(),
            extensions: str_vec(["ase", "xcf"]),
            viewer: "binary-source".into(),
            icon: Some("pixel".into()),
            is_source: true,
        },
        AssetType {
            kind: "source_design".into(),
            label: "设计源".into(),
            extensions: str_vec(["psd", "ai"]),
            viewer: "binary-source".into(),
            icon: Some("design".into()),
            is_source: true,
        },
        AssetType {
            kind: "archive".into(),
            label: "压缩包".into(),
            extensions: str_vec(["zip", "7z", "rar"]),
            viewer: "fallback".into(),
            icon: Some("archive".into()),
            is_source: false,
        },
        AssetType {
            kind: "legacy_media".into(),
            label: "旧媒体".into(),
            extensions: str_vec(["swf"]),
            viewer: "fallback".into(),
            icon: Some("legacy".into()),
            is_source: false,
        },
        AssetType {
            kind: "other".into(),
            label: "其他".into(),
            extensions: vec![],
            viewer: "fallback".into(),
            icon: Some("file".into()),
            is_source: false,
        },
    ]
}

fn str_vec<const N: usize>(arr: [&str; N]) -> Vec<String> {
    arr.iter().map(|s| s.to_string()).collect()
}

/// 合并后的注册表：扩展名(小写) → AssetType。
pub struct Registry {
    by_ext: HashMap<String, AssetType>,
    types: Vec<AssetType>,
}

impl Registry {
    /// 加载：内置默认为底，再用 asset_types 表覆盖/追加。
    pub async fn load(pool: &SqlitePool) -> Result<Self, sqlx::Error> {
        let mut types = builtin_types();
        // 读数据库覆盖/新增项
        let rows: Vec<(String, String, String, String, Option<String>, i64)> =
            sqlx::query_as("SELECT kind,label,extensions,viewer,icon,is_source FROM asset_types")
                .fetch_all(pool)
                .await?;
        for (kind, label, exts_json, viewer, icon, is_source) in rows {
            let extensions: Vec<String> = serde_json::from_str(&exts_json).unwrap_or_default();
            let at = AssetType {
                kind: kind.clone(),
                label,
                extensions,
                viewer,
                icon,
                is_source: is_source != 0,
            };
            if let Some(pos) = types.iter().position(|t| t.kind == kind) {
                types[pos] = at; // 覆盖内置
            } else {
                types.push(at); // 用户新增
            }
        }
        // 建 扩展名→类型 索引
        let mut by_ext = HashMap::new();
        for t in &types {
            for e in &t.extensions {
                by_ext.insert(e.to_lowercase(), t.clone());
            }
        }
        Ok(Registry { by_ext, types })
    }

    /// 按扩展名查 kind；未命中返回 "other"。
    pub fn kind_for(&self, ext: &str) -> &str {
        match self.by_ext.get(&ext.to_lowercase()) {
            Some(t) => &t.kind,
            None => "other",
        }
    }

    pub fn all(&self) -> &[AssetType] {
        &self.types
    }

    /// 从类型列表直接构造（测试用，无需数据库）。
    pub fn from_types(types: Vec<AssetType>) -> Self {
        let mut by_ext = HashMap::new();
        for t in &types {
            for e in &t.extensions {
                by_ext.insert(e.to_lowercase(), t.clone());
            }
        }
        Registry { by_ext, types }
    }
}

use tauri::State;

/// 列出合并后的全表（内置默认 + 数据库覆盖）
#[tauri::command]
pub async fn list_asset_types(pool: State<'_, SqlitePool>) -> Result<Vec<AssetType>, AppError> {
    let reg = Registry::load(&pool).await?;
    Ok(reg.all().to_vec())
}

/// 新增/编辑类型（覆盖内置或追加自定义）
#[tauri::command]
pub async fn upsert_asset_type(
    kind: String,
    label: String,
    extensions: Vec<String>,
    viewer: String,
    is_source: bool,
    built_in: bool,
    pool: State<'_, SqlitePool>,
) -> Result<(), AppError> {
    let exts_json = serde_json::to_string(&extensions)?;
    sqlx::query(
        "INSERT INTO asset_types(kind,label,extensions,viewer,is_source,built_in)
         VALUES(?,?,?,?,?,?)
         ON CONFLICT(kind) DO UPDATE SET label=excluded.label, extensions=excluded.extensions,
           viewer=excluded.viewer, is_source=excluded.is_source",
    )
    .bind(&kind)
    .bind(&label)
    .bind(&exts_json)
    .bind(&viewer)
    .bind(if is_source { 1 } else { 0 })
    .bind(if built_in { 1 } else { 0 })
    .execute(&*pool)
    .await
    ?;
    Ok(())
}

/// 删除类型（仅限用户新增项 built_in=0）
#[tauri::command]
pub async fn delete_asset_type(kind: String, pool: State<'_, SqlitePool>) -> Result<(), AppError> {
    let res = sqlx::query("DELETE FROM asset_types WHERE kind=? AND built_in=0")
        .bind(&kind)
        .execute(&*pool)
        .await
        ?;
    if res.rows_affected() == 0 {
        return Err(AppError::InvalidInput(
            "内置类型不可删除（只能覆盖编辑）".to_string(),
        ));
    }
    Ok(())
}

/// 按当前注册表重新分类全库（重算 files.kind）
#[derive(serde::Serialize)]
pub struct ReclassifyReport {
    pub updated: i64,
}

#[tauri::command]
pub async fn reclassify_all(
    app: tauri::AppHandle,
    pool: State<'_, SqlitePool>,
) -> Result<ReclassifyReport, AppError> {
    use tauri::Emitter;
    let reg = Registry::load(&pool).await?;
    let files: Vec<(i64, String)> = sqlx::query_as("SELECT id, ext FROM files WHERE deleted=0")
        .fetch_all(&*pool)
        .await
        ?;
    let total = files.len() as u64;
    let mut updated = 0i64;
    let mut done = 0u64;
    // 单事务批量更新
    let mut tx = pool.begin().await?;
    for (id, ext) in &files {
        let new_kind = reg.kind_for(ext);
        let res = sqlx::query("UPDATE files SET kind=? WHERE id=? AND kind!=?")
            .bind(new_kind)
            .bind(id)
            .bind(new_kind)
            .execute(&mut *tx)
            .await
            ?;
        if res.rows_affected() > 0 {
            updated += 1;
        }
        done += 1;
        if done % 2000 == 0 {
            let _ = app.emit(
                "reclassify://progress",
                serde_json::json!({ "done": done, "total": total }),
            );
        }
    }
    tx.commit().await?;
    let _ = app.emit(
        "reclassify://progress",
        serde_json::json!({ "done": done, "total": total }),
    );
    Ok(ReclassifyReport { updated })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// kind_for：内置类型扩展名命中
    #[test]
    fn test_kind_for_builtin() {
        let reg = Registry::from_types(builtin_types());
        assert_eq!(reg.kind_for("png"), "image");
        assert_eq!(reg.kind_for("JPG"), "image"); // 大写应命中
        assert_eq!(reg.kind_for("gif"), "animated");
        assert_eq!(reg.kind_for("ogg"), "audio");
        assert_eq!(reg.kind_for("ttf"), "font");
        assert_eq!(reg.kind_for("glb"), "model3d");
    }

    #[test]
    fn test_kind_for_unknown() {
        let reg = Registry::from_types(builtin_types());
        assert_eq!(reg.kind_for("xyz"), "other");
        assert_eq!(reg.kind_for(""), "other");
    }

    /// 自定义类型覆盖：用户新增 video 类型后能命中
    #[test]
    fn test_kind_for_custom_type() {
        let mut types = builtin_types();
        types.push(AssetType {
            kind: "video".into(),
            label: "视频".into(),
            extensions: vec!["mp4".into(), "webm".into()],
            viewer: "fallback".into(),
            icon: None,
            is_source: false,
        });
        let reg = Registry::from_types(types);
        assert_eq!(reg.kind_for("mp4"), "video");
        assert_eq!(reg.kind_for("webm"), "video");
        // 原有类型不受影响
        assert_eq!(reg.kind_for("png"), "image");
    }

    #[test]
    fn test_kind_for_case_insensitive() {
        let reg = Registry::from_types(builtin_types());
        // 大小写不敏感
        assert_eq!(reg.kind_for("PNG"), reg.kind_for("png"));
        assert_eq!(reg.kind_for("Wav"), "audio");
    }
}

