use rayon::prelude::*;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// 一条扫描结果。
#[derive(Debug, Clone)]
pub struct ScanEntry {
    pub category: String, // 一级目录名
    pub package: String,  // 二级目录名(包)
    pub rel_path: String, // 相对包的路径(文件)
    pub name: String,
    pub ext: String,
    pub bytes: u64,
    pub modified_at: i64,
}

/// 需跳过的目录名(精确匹配)。
const SKIP_DIRS: &[&str] = &["_下载脚本"];

fn should_skip(name: &str) -> bool {
    SKIP_DIRS.contains(&name) || name.starts_with('.')
}

/// 扫描整个库。分类=一级目录，包=二级目录，文件=更深。
pub fn scan_library(root: &Path) -> Vec<ScanEntry> {
    let categories: Vec<PathBuf> = std::fs::read_dir(root)
        .expect("read root")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.is_dir()
                && !should_skip(p.file_name().and_then(|n| n.to_str()).unwrap_or(""))
        })
        .collect();

    // 每个分类并行扫描
    let entries: Vec<Vec<ScanEntry>> = categories
        .par_iter()
        .map(|cat_dir| {
            let cat_name = cat_dir
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();
            // 该分类下的包(二级目录)
            let packages: Vec<PathBuf> = std::fs::read_dir(cat_dir)
                .expect("read category")
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| {
                    p.is_dir()
                        && !should_skip(p.file_name().and_then(|n| n.to_str()).unwrap_or(""))
                })
                .collect();
            packages
                .iter()
                .flat_map(|pkg_dir| {
                    let pkg_name = pkg_dir
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("")
                        .to_string();
                    WalkDir::new(pkg_dir)
                        .into_iter()
                        .filter_map(|e| e.ok())
                        .filter_map(|entry| {
                            if !entry.file_type().is_file() {
                                return None;
                            }
                            let meta = entry.metadata().ok()?;
                            let full = entry.path();
                            let rel = full
                                .strip_prefix(pkg_dir)
                                .ok()?
                                .to_string_lossy()
                                .replace('\\', "/");
                            let name = entry.file_name().to_string_lossy().to_string();
                            let ext = full
                                .extension()
                                .and_then(|e| e.to_str())
                                .map(|s| s.to_lowercase())
                                .unwrap_or_default();
                            let modified = meta
                                .modified()
                                .ok()
                                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                                .map(|d| d.as_secs() as i64)
                                .unwrap_or(0);
                            Some(ScanEntry {
                                category: cat_name.clone(),
                                package: pkg_name.clone(),
                                rel_path: rel,
                                name,
                                ext,
                                bytes: meta.len(),
                                modified_at: modified,
                            })
                        })
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>()
        })
        .collect();

    entries.into_iter().flatten().collect()
}
