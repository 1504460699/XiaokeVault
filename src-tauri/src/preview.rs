use serde::Serialize;
use std::path::PathBuf;
use std::process::Command;

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
