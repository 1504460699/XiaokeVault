use crate::error::AppError;
use serde::Serialize;
use sqlx::SqlitePool;
use std::collections::HashMap;
use tauri::State;

use crate::library::FileNode;

/// 树节点（递归结构）
#[derive(Debug, Serialize)]
pub struct DirNode {
    pub id: i64,
    pub name: String,
    pub path: String,
    pub depth: i32,
    pub file_count: i64,
    pub total_bytes: i64,
    pub children: Vec<DirNode>,
}

/// 取整棵目录树（嵌套 children）
#[tauri::command]
pub async fn get_directory_tree(
    lib_id: i64,
    pool: State<'_, SqlitePool>,
) -> Result<Vec<DirNode>, AppError> {
    // 一次取所有目录行，内存里组装树
    let rows: Vec<(i64, Option<i64>, String, String, i32, i64, i64)> = sqlx::query_as(
        "SELECT id, parent_id, name, path, depth, file_count, total_bytes
         FROM directories WHERE library_id=? ORDER BY depth, name",
    )
    .bind(lib_id)
    .fetch_all(&*pool)
    .await
    ?;

    // id -> DirNode（先建叶子，再挂到父）
    let mut nodes: HashMap<i64, DirNode> = HashMap::new();
    let mut parent_map: HashMap<i64, Option<i64>> = HashMap::new();
    for (id, parent_id, name, path, depth, fc, tb) in rows {
        nodes.insert(
            id,
            DirNode {
                id,
                name,
                path,
                depth,
                file_count: fc,
                total_bytes: tb,
                children: vec![],
            },
        );
        parent_map.insert(id, parent_id);
    }
    // 收集父子关系
    let mut child_map: HashMap<i64, Vec<i64>> = HashMap::new();
    for (id, parent_id) in &parent_map {
        if let Some(pid) = parent_id {
            child_map.entry(*pid).or_default().push(*id);
        }
    }
    // 递归构建
    fn build(id: i64, nodes: &mut HashMap<i64, DirNode>, child_map: &HashMap<i64, Vec<i64>>) -> DirNode {
        let mut node = nodes.remove(&id).unwrap();
        if let Some(child_ids) = child_map.get(&id) {
            for cid in child_ids {
                node.children.push(build(*cid, nodes, child_map));
            }
        }
        node
    }
    let mut roots: Vec<DirNode> = Vec::new();
    for (id, parent_id) in &parent_map {
        if parent_id.is_none() {
            roots.push(build(*id, &mut nodes, &child_map));
        }
    }
    roots.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(roots)
}

/// 取某目录【直接】含的文件
#[tauri::command]
pub async fn get_directory_files(
    directory_id: i64,
    pool: State<'_, SqlitePool>,
) -> Result<Vec<FileNode>, AppError> {
    // directory.path 是相对库根的，需库根拼绝对路径
    let rows: Vec<(i64, String, String, String, String, i64, String, String)> = sqlx::query_as(
        "SELECT f.id, f.rel_path, f.name, f.ext, f.kind, f.bytes, d.path, l.root_path
         FROM files f
         JOIN directories d ON d.id=f.directory_id
         JOIN libraries l ON l.id=d.library_id
         WHERE f.directory_id=? AND f.deleted=0 ORDER BY f.rel_path",
    )
    .bind(directory_id)
    .fetch_all(&*pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(id, rel, name, ext, kind, bytes, dir_path, root)| FileNode {
            id,
            rel_path: rel.clone(),
            name,
            ext,
            kind,
            bytes,
            abs_path: format!("{}/{}/{}", root.replace('\\', "/"), dir_path, rel),
        })
        .collect())
}

/// 取某目录及所有子目录的文件汇总（递归 CTE）
#[tauri::command]
pub async fn get_subtree_files(
    directory_id: i64,
    pool: State<'_, SqlitePool>,
) -> Result<Vec<FileNode>, AppError> {
    // directory.path 相对库根，需库根拼绝对路径
    let rows: Vec<(i64, String, String, String, String, i64, String, String)> = sqlx::query_as(
        "WITH RECURSIVE desc(id) AS (
            SELECT id FROM directories WHERE id=?
            UNION ALL
            SELECT d.id FROM directories d JOIN desc ON d.parent_id=desc.id
         )
         SELECT f.id, f.rel_path, f.name, f.ext, f.kind, f.bytes, dir.path, l.root_path
         FROM files f
         JOIN directories dir ON dir.id=f.directory_id
         JOIN libraries l ON l.id=dir.library_id
         WHERE f.directory_id IN (SELECT id FROM desc) AND f.deleted=0
         ORDER BY f.rel_path",
    )
    .bind(directory_id)
    .fetch_all(&*pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(id, rel, name, ext, kind, bytes, dir_path, root)| FileNode {
            id,
            rel_path: rel.clone(),
            name,
            ext,
            kind,
            bytes,
            abs_path: format!("{}/{}/{}", root.replace('\\', "/"), dir_path, rel),
        })
        .collect())
}

/// 取整库所有文件（点击库根节点时用）。覆盖树视图所有目录。
#[tauri::command]
pub async fn get_all_library_files(
    library_id: i64,
    pool: State<'_, SqlitePool>,
) -> Result<Vec<FileNode>, AppError> {
    let rows: Vec<(i64, String, String, String, String, i64, String, String)> = sqlx::query_as(
        "SELECT f.id, f.rel_path, f.name, f.ext, f.kind, f.bytes, d.path, l.root_path
         FROM files f
         JOIN directories d ON d.id=f.directory_id
         JOIN libraries l ON l.id=d.library_id
         WHERE d.library_id=? AND f.deleted=0
         ORDER BY f.rel_path",
    )
    .bind(library_id)
    .fetch_all(&*pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(id, rel, name, ext, kind, bytes, dir_path, root)| FileNode {
            id,
            rel_path: rel.clone(),
            name,
            ext,
            kind,
            bytes,
            abs_path: format!("{}/{}/{}", root.replace('\\', "/"), dir_path, rel),
        })
        .collect())
}
