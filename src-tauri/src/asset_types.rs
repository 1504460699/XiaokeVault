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
}
