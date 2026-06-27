use rayon::prelude::*;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// 跳过的目录名（与 scanner.rs 一致）
const SKIP_DIRS: &[&str] = &["_下载脚本"];

fn should_skip(name: &str) -> bool {
    SKIP_DIRS.contains(&name) || name.starts_with('.')
}

/// 一个目录节点（扫描结果）
#[derive(Debug, Clone)]
pub struct DirScanEntry {
    pub path: String, // 相对库根，如 "05_自然环境/树木"
    pub name: String, // 目录名，如 "树木"
    pub depth: i32,
    pub parent_path: Option<String>, // None=根级
}

/// 一个文件（扫描结果，挂在直接所在目录）
#[derive(Debug, Clone)]
pub struct TreeFileEntry {
    pub dir_path: String, // 直接所在目录的相对路径
    pub rel_path: String, // 相对该目录的路径
    pub name: String,
    pub ext: String,
    pub bytes: u64,
    pub modified_at: i64,
}

pub struct TreeScanResult {
    pub dirs: Vec<DirScanEntry>,
    pub files: Vec<TreeFileEntry>,
}

/// 完整遍历目录树。返回所有目录 + 文件，按真实结构。
pub fn scan_library_tree(root: &Path) -> TreeScanResult {
    // 收集库根下所有顶层目录作为扫描起点（并行）
    let top_dirs: Vec<PathBuf> = std::fs::read_dir(root)
        .expect("read root")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.is_dir() && !should_skip(p.file_name().and_then(|n| n.to_str()).unwrap_or(""))
        })
        .collect();

    // 并行扫描每个顶层目录
    let results: Vec<TreeScanResult> = top_dirs
        .par_iter()
        .map(|top| scan_subtree(root, top))
        .collect();

    let mut dirs = Vec::new();
    let mut files = Vec::new();
    for r in results {
        dirs.extend(r.dirs);
        files.extend(r.files);
    }
    TreeScanResult { dirs, files }
}

/// 递归扫描一个子树（top 相对 root 是顶层目录）
fn scan_subtree(root: &Path, top: &Path) -> TreeScanResult {
    let mut dirs = Vec::new();
    let mut files = Vec::new();

    for entry in WalkDir::new(top).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        let rel = match path.strip_prefix(root) {
            Ok(r) => r.to_string_lossy().replace('\\', "/"),
            Err(_) => continue,
        };
        let ft = entry.file_type();

        if ft.is_dir() {
            // 跳过隐藏/下载脚本目录（非顶层也要检查）
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if entry.depth() > 0 && should_skip(name) {
                continue;
            }
            let depth = entry.depth();
            let parent_path = if depth == 0 {
                None
            } else {
                path.parent()
                    .and_then(|p| p.strip_prefix(root).ok())
                    .map(|p| p.to_string_lossy().replace('\\', "/"))
            };
            dirs.push(DirScanEntry {
                path: rel,
                name: name.to_string(),
                depth: depth as i32,
                parent_path,
            });
        } else if ft.is_file() {
            let meta = match entry.metadata() {
                Ok(m) => m,
                Err(_) => continue,
            };
            // 直接所在目录（相对库根）
            let dir_path = path
                .parent()
                .and_then(|p| p.strip_prefix(root).ok())
                .map(|p| p.to_string_lossy().replace('\\', "/"))
                .unwrap_or_default();
            // 相对该目录的路径
            let rel_in_dir = path
                .strip_prefix(root.join(&dir_path))
                .ok()
                .map(|p| p.to_string_lossy().replace('\\', "/"))
                .unwrap_or_default();
            let name = entry.file_name().to_string_lossy().to_string();
            let ext = path
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
            files.push(TreeFileEntry {
                dir_path,
                rel_path: rel_in_dir,
                name,
                ext,
                bytes: meta.len(),
                modified_at: modified,
            });
        }
    }
    TreeScanResult { dirs, files }
}
