use serde::Serialize;
use sqlx::SqlitePool;
use std::path::PathBuf;
use std::process::Command;
use tauri::State;

const THUMB_SIZE: u32 = 200;     // 缩略图最大边
const LARGE_BYTES: i64 = 51200;  // >50KB 才生成缩略图

/// 缩略图结果
#[derive(Debug, Serialize)]
pub struct ThumbResult {
    pub path: String,    // 缩略图路径或原图路径
    pub is_thumb: bool,  // 是否为生成的缩略图
}

/// 返回缩略图路径：大图(>50KB)生成缩略图缓存，小图直接返回原路径
#[tauri::command]
pub async fn get_thumbnail(file_id: i64, pool: State<'_, SqlitePool>) -> Result<ThumbResult, String> {
    let (abs_pkg, rel, bytes, ext): (String, String, i64, String) = sqlx::query_as(
        "SELECT p.path, f.rel_path, f.bytes, f.ext FROM files f
         JOIN packages p ON p.id=f.package_id WHERE f.id=?",
    )
    .bind(file_id)
    .fetch_one(&*pool)
    .await
    .map_err(|e| e.to_string())?;

    let src = std::path::Path::new(&abs_pkg).join(&rel);

    // 小图直接返回原路径
    if bytes < LARGE_BYTES {
        return Ok(ThumbResult {
            path: src.to_string_lossy().to_string(),
            is_thumb: false,
        });
    }

    // 大图：生成缩略图
    let mut cache = dirs::data_dir().ok_or("no data dir")?;
    cache.push("com.xiaoke.tauri-app");
    cache.push("thumbs");
    std::fs::create_dir_all(&cache).map_err(|e| e.to_string())?;
    let thumb_path = cache.join(format!("{}.webp", file_id));

    // 缓存命中
    if thumb_path.exists() {
        return Ok(ThumbResult {
            path: thumb_path.to_string_lossy().to_string(),
            is_thumb: true,
        });
    }

    // 生成缩略图
    let _ = ext; // 扩展名 image crate 自动识别
    match generate_thumb(&src, &thumb_path) {
        Ok(()) => Ok(ThumbResult {
            path: thumb_path.to_string_lossy().to_string(),
            is_thumb: true,
        }),
        Err(_) => {
            // 生成失败则降级返回原图
            Ok(ThumbResult {
                path: src.to_string_lossy().to_string(),
                is_thumb: false,
            })
        }
    }
}

fn generate_thumb(src: &std::path::Path, dest: &std::path::Path) -> Result<(), String> {
    let img = image::open(src).map_err(|e| e.to_string())?;
    let thumb = img.resize(THUMB_SIZE, THUMB_SIZE, image::imageops::FilterType::Nearest);
    thumb.save_with_format(dest, image::ImageFormat::WebP).map_err(|e| e.to_string())?;
    Ok(())
}

#[derive(Debug, Serialize)]
pub struct ModelPath {
    pub path: String,
    pub source: String, // "blender" / "error"
    pub message: String,
}

/// 检测本机 Blender 路径
fn find_blender() -> Option<PathBuf> {
    let candidates = [
        r"C:\Program Files\Blender Foundation\Blender\blender.exe",
        r"C:\Program Files\Blender Foundation\Blender 4.2\blender.exe",
        r"C:\Program Files\Blender Foundation\Blender 4.3\blender.exe",
        r"C:\Program Files\Blender Foundation\Blender 4.1\blender.exe",
        r"C:\Program Files\Blender Foundation\Blender 3.6\blender.exe",
    ];
    for c in candidates {
        if std::path::Path::new(c).exists() {
            return Some(PathBuf::from(c));
        }
    }
    if Command::new("blender").arg("--version").output().is_ok() {
        return Some(PathBuf::from("blender"));
    }
    None
}

/// 返回 blend 转换后的 glb 路径（带缓存）
#[tauri::command]
pub async fn get_model_glb(blend_path: String) -> Result<ModelPath, String> {
    let mut cache = dirs::data_dir().ok_or("no data dir")?;
    cache.push("com.xiaoke.tauri-app");
    cache.push("glb_cache");
    std::fs::create_dir_all(&cache).map_err(|e| e.to_string())?;
    let stem = std::path::Path::new(&blend_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("model");
    let glb = cache.join(format!("{}.glb", stem));
    if glb.exists() {
        return Ok(ModelPath {
            path: glb.to_string_lossy().to_string(),
            source: "blender".into(),
            message: "缓存命中".into(),
        });
    }
    let blender = match find_blender() {
        Some(b) => b,
        None => {
            return Ok(ModelPath {
                path: String::new(),
                source: "error".into(),
                message: "未检测到本机 Blender，请安装 Blender (https://www.blender.org)".into(),
            })
        }
    };
    let script = cache.join("export_glb.py");
    std::fs::write(&script, EXPORT_SCRIPT).map_err(|e| e.to_string())?;
    let out = Command::new(&blender)
        .arg("--background")
        .arg(&blend_path)
        .arg("--python")
        .arg(&script)
        .arg("--")
        .arg(&glb)
        .output()
        .map_err(|e| e.to_string())?;
    if !out.status.success() || !glb.exists() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        return Ok(ModelPath {
            path: String::new(),
            source: "error".into(),
            message: format!("Blender 转换失败：{}", &stderr[..stderr.len().min(200)]),
        });
    }
    Ok(ModelPath {
        path: glb.to_string_lossy().to_string(),
        source: "blender".into(),
        message: "转换成功".into(),
    })
}

const EXPORT_SCRIPT: &str = r#"
import bpy, sys
out = sys.argv[-1]
bpy.ops.export_scene.gltf(filepath=out, export_format='GLB')
"#;
