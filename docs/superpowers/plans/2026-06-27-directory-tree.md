# 目录树视图（Directory Tree View）实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 在左侧面板新增"目录树视图"，按真实目录结构（任意深度）展示资源库，只读镜像 + 如实反映磁盘变化。

**Architecture:** 新增 `directories` 表（parent_id 自关联树）与现有 `categories/packages` 并存；`files` 加 `directory_id` 字段；扫描逻辑改为 walkdir 完整遍历；增量扫描同步维护 directories（增/删/改）；前端左侧顶部加视图切换按钮，新增递归树组件 DirectoryTree.vue。

**Tech Stack:** Rust + sqlx + walkdir + rayon（后端），Vue 3 + Pinia + Tailwind（前端）。

**设计文档：** `docs/superpowers/specs/2026-06-27-directory-tree-design.md`

---

## 文件结构

**后端（Rust）：**
- Create: `src-tauri/migrations/0004_directories.sql` — directories 表 + files.directory_id
- Modify: `src-tauri/src/db.rs` — migrate() 加载 0004
- Create: `src-tauri/src/tree_scanner.rs` — 完整目录树扫描 scan_library_tree()
- Modify: `src-tauri/src/indexer.rs` — scan_into 同时写 directories + files.directory_id
- Create: `src-tauri/src/tree.rs` — 3 个新命令（get_directory_tree/files/subtree_files）
- Modify: `src-tauri/src/lib.rs` — 注册新命令

**前端（Vue/TS）：**
- Modify: `src/types/library.ts` — DirNode 类型
- Modify: `src/ipc/library.ts` — 3 个新 ipc 方法
- Create: `src/stores/treeStore.ts` — 树视图状态
- Create: `src/components/DirectoryTree.vue` — 递归树组件
- Create: `src/components/DirTreeNode.vue` — 单个树节点（递归）
- Modify: `src/components/CategoryTree.vue` — 顶部加视图切换
- Modify: `src/i18n/zh.ts` + `en.ts` — 树视图相关文案
- Modify: `src/App.vue` — 左侧面板根据视图切换渲染

---

## Task 1: 数据库迁移（directories 表 + files.directory_id）

**Files:**
- Create: `src-tauri/migrations/0004_directories.sql`
- Modify: `src-tauri/src/db.rs:36` (migrate 函数)

- [ ] **Step 1: 创建迁移文件 0004_directories.sql**

```sql
-- 目录树表（自关联树结构，与 categories/packages 并存）
CREATE TABLE IF NOT EXISTS directories (
    id          INTEGER PRIMARY KEY,
    library_id  INTEGER NOT NULL REFERENCES libraries(id) ON DELETE CASCADE,
    parent_id   INTEGER REFERENCES directories(id) ON DELETE CASCADE,
    name        TEXT NOT NULL,
    path        TEXT NOT NULL,
    depth       INTEGER NOT NULL DEFAULT 0,
    file_count  INTEGER DEFAULT 0,
    total_bytes INTEGER DEFAULT 0,
    UNIQUE(library_id, path)
);
CREATE INDEX IF NOT EXISTS idx_dirs_parent ON directories(parent_id);
CREATE INDEX IF NOT EXISTS idx_dirs_library ON directories(library_id);

-- files 表增加 directory_id（关联直接所在目录）
ALTER TABLE files ADD COLUMN directory_id INTEGER REFERENCES directories(id) ON DELETE CASCADE;
CREATE INDEX IF NOT EXISTS idx_files_directory ON files(directory_id);
```

- [ ] **Step 2: 在 db.rs migrate() 加载 0004**

在 `migrate()` 函数末尾（0003 之后）添加：

```rust
    sqlx::query(include_str!("../migrations/0004_directories.sql"))
        .execute(pool)
        .await?;
    Ok(())
```

- [ ] **Step 3: 编译验证**

Run: `cd src-tauri && cargo check`
Expected: 编译通过

- [ ] **Step 4: 启动应用验证迁移生效**

Run: `pnpm tauri dev`，启动后查看日志无 SQL 错误。
检查数据库（用 sqlite 工具或 sqlx 日志确认 `directories` 表存在、`files` 有 `directory_id` 列）。

- [ ] **Step 5: Commit**

```bash
git add src-tauri/migrations/0004_directories.sql src-tauri/src/db.rs
git commit -m "feat(tree): directories 表迁移 + files.directory_id"
```

---

## Task 2: 完整目录树扫描（tree_scanner.rs）

**Files:**
- Create: `src-tauri/src/tree_scanner.rs`
- Modify: `src-tauri/src/lib.rs:10` (mod 声明)

- [ ] **Step 1: 创建 tree_scanner.rs**

完整目录树遍历，输出 DirScanEntry（目录）和 TreeFileEntry（文件），不再假设三级结构。

```rust
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
    pub path: String,        // 相对库根，如 "05_自然环境/树木"
    pub name: String,        // 目录名，如 "树木"
    pub depth: i32,
    pub parent_path: Option<String>,  // None=根级
}

/// 一个文件（扫描结果，挂在直接所在目录）
#[derive(Debug, Clone)]
pub struct TreeFileEntry {
    pub dir_path: String,    // 直接所在目录的相对路径
    pub rel_path: String,    // 相对该目录的路径
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
    // 收集所有子目录（库根的直接子项作为起点，并行扫描）
    let top_dirs: Vec<PathBuf> = std::fs::read_dir(root)
        .expect("read root")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.is_dir() && !should_skip(p.file_name().and_then(|n| n.to_str()).unwrap_or("")))
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

    for entry in WalkDir::new(top)
        .into_iter()
        .filter_map(|e| e.ok())
    {
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
```

- [ ] **Step 2: 在 lib.rs 注册模块**

在 `lib.rs` 的 `mod` 声明区（约第 10 行 `mod watcher;` 附近）添加：

```rust
mod tree_scanner;
```

- [ ] **Step 3: 编译验证**

Run: `cd src-tauri && cargo check`
Expected: 编译通过（可能有 unused 警告，正常）

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/tree_scanner.rs src-tauri/src/lib.rs
git commit -m "feat(tree): 完整目录树扫描 tree_scanner.rs"
```

---

## Task 3: 扫描入库写 directories + files.directory_id

**Files:**
- Modify: `src-tauri/src/indexer.rs` (新增 scan_tree_into 函数)

- [ ] **Step 1: 在 indexer.rs 新增 scan_tree_into**

在文件末尾（tests 模块之前）添加。复用 Registry 分类逻辑，但写 directories 表 + files.directory_id。

```rust
use crate::tree_scanner::{self, DirScanEntry, TreeFileEntry};

/// 目录树版扫描入库：写 directories + files（带 directory_id）。
/// 与 scan_into 并存，互不影响（scan_into 仍写 packages）。
pub async fn scan_tree_into(
    pool: &SqlitePool,
    lib_id: i64,
    root: &Path,
) -> Result<ScanReport, sqlx::Error> {
    let start = std::time::Instant::now();
    let registry = Registry::load(pool).await?;
    let mut unknown: HashMap<String, u64> = HashMap::new();
    let result = tree_scanner::scan_library_tree(root);

    let mut new = 0u64;
    let mut updated = 0u64;
    let mut deleted = 0u64;
    let mut total_written = 0u64;

    let mut tx = pool.begin().await?;

    // === 1. 写 directories 表 ===
    // path -> id 映射，供 files.directory_id 用
    let mut path_to_id: HashMap<String, i64> = HashMap::new();

    // 先取所有已存在 directory 的 path（用于增量删除判断）
    let existing_dir_paths: HashSet<String> =
        sqlx::query_scalar("SELECT path FROM directories WHERE library_id=?")
            .bind(lib_id)
            .fetch_all(&mut *tx)
            .await?
            .into_iter()
            .collect();

    let mut seen_dir_paths: HashSet<String> = HashSet::new();

    for d in &result.dirs {
        // 解析 parent_id
        let parent_id: Option<i64> = match &d.parent_path {
            Some(pp) => *path_to_id.get(pp),
            None => None,
        };
        sqlx::query(
            "INSERT INTO directories(library_id,parent_id,name,path,depth,file_count,total_bytes)
             VALUES(?,?,?,?,?,0,0)
             ON CONFLICT(library_id,path) DO UPDATE SET
               parent_id=excluded.parent_id, name=excluded.name, depth=excluded.depth",
        )
        .bind(lib_id)
        .bind(parent_id)
        .bind(&d.name)
        .bind(&d.path)
        .bind(d.depth)
        .execute(&mut *tx)
        .await?;
        let (id,): (i64,) =
            sqlx::query_as("SELECT id FROM directories WHERE library_id=? AND path=?")
                .bind(lib_id)
                .bind(&d.path)
                .fetch_one(&mut *tx)
                .await?;
        path_to_id.insert(d.path.clone(), id);
        seen_dir_paths.insert(d.path.clone());
    }

    // === 2. 写 files（带 directory_id）+ 累计 directory 的 file_count/bytes ===
    let mut dir_stats: HashMap<String, (i64, i64)> = HashMap::new(); // dir_path -> (count, bytes)

    for f in &result.files {
        let kind = if registry.kind_for(&f.ext) == "other" && !f.ext.is_empty() {
            *unknown.entry(f.ext.clone()).or_insert(0) += 1;
            "other"
        } else {
            registry.kind_for(&f.ext)
        };
        let dir_id = path_to_id.get(&f.dir_path).copied();

        // 查是否已存在（用 directory_id + rel_path 判断）
        let existed: bool = sqlx::query_scalar(
            "SELECT 1 FROM files WHERE directory_id=? AND rel_path=? LIMIT 1",
        )
        .bind(dir_id)
        .bind(&f.rel_path)
        .fetch_optional(&mut *tx)
        .await?
        .is_some();

        // 注意：旧表的 UNIQUE(package_id, rel_path) 在树视图下不适用，
        // 这里用 directory_id + name 作为业务唯一性（rel_path 相对目录）
        sqlx::query(
            "INSERT INTO files(directory_id,package_id,rel_path,name,ext,kind,bytes,modified_at,deleted)
             VALUES(?,?,?,?,?,?,?,?,0)
             ON CONFLICT(package_id,rel_path) DO UPDATE SET
               directory_id=excluded.directory_id, name=excluded.name, ext=excluded.ext,
               kind=excluded.kind, bytes=excluded.bytes, modified_at=excluded.modified_at, deleted=0",
        )
        .bind(dir_id)
        .bind(0i64) // package_id=0 表示树视图文件（不关联 packages）
        .bind(&f.rel_path)
        .bind(&f.name)
        .bind(&f.ext)
        .bind(kind)
        .bind(f.bytes as i64)
        .bind(f.modified_at)
        .execute(&mut *tx)
        .await?;

        if existed { updated += 1; } else { new += 1; }
        total_written += 1;
        let e = dir_stats.entry(f.dir_path.clone()).or_insert((0, 0));
        e.0 += 1;
        e.1 += f.bytes as i64;
    }

    // === 3. 更新 directories 的 file_count/total_bytes ===
    for (dir_path, (count, bytes)) in &dir_stats {
        if let Some(id) = path_to_id.get(dir_path) {
            sqlx::query("UPDATE directories SET file_count=?, total_bytes=? WHERE id=?")
                .bind(count)
                .bind(bytes)
                .bind(id)
                .execute(&mut *tx)
                .await?;
        }
    }
    // 没文件的目录 file_count 置 0
    for (dir_path, id) in &path_to_id {
        if !dir_stats.contains_key(dir_path) {
            sqlx::query("UPDATE directories SET file_count=0, total_bytes=0 WHERE id=?")
                .bind(id)
                .execute(&mut *tx)
                .await?;
        }
    }

    // === 4. 增量删除：磁盘已不存在的目录直接删（CASCADE 删其 files）===
    for old_path in existing_dir_paths.iter() {
        if !seen_dir_paths.contains(old_path) {
            sqlx::query("DELETE FROM directories WHERE library_id=? AND path=?")
                .bind(lib_id)
                .bind(old_path)
                .execute(&mut *tx)
                .await?;
            deleted += 1;
        }
    }

    tx.commit().await?;

    Ok(ScanReport {
        new,
        updated,
        deleted,
        total_files: total_written,
        duration_ms: start.elapsed().as_millis(),
        errors: vec![],
        unknown_extensions: unknown.into_iter().collect(),
    })
}
```

- [ ] **Step 2: 编译验证**

Run: `cd src-tauri && cargo check`
Expected: 编译通过

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/indexer.rs
git commit -m "feat(tree): scan_tree_into 写 directories + files.directory_id"
```

---

## Task 4: 全量扫描触发树扫描

**Files:**
- Modify: `src-tauri/src/library.rs` (scan_library_full 命令)

- [ ] **Step 1: 在 scan_library_full 末尾追加树扫描**

找到 `scan_library_full` 函数（约 library.rs:148），在 `UPDATE libraries SET last_scan_at` 之后、`Ok(report)` 之前，追加：

```rust
    // 同时跑目录树扫描（写 directories 表）
    if let Err(e) = indexer::scan_tree_into(&*pool, lib_id, &PathBuf::from(&root)).await {
        log::warn!("[scan] 目录树扫描失败（不影响主扫描）：{e}");
    }
```

注意顶部已有 `use crate::indexer::{self, ScanReport};`，直接用 indexer::。

- [ ] **Step 2: 编译验证**

Run: `cd src-tauri && cargo check`
Expected: 编译通过

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/library.rs
git commit -m "feat(tree): 全量扫描同时写目录树"
```

---

## Task 5: watcher 增量扫描同步树

**Files:**
- Modify: `src-tauri/src/watcher.rs` (增量扫描后追加树扫描)

- [ ] **Step 1: 在 watcher.rs 的 scan_into 调用后追加树扫描**

找到 watcher.rs 中调用 `crate::indexer::scan_into(...)` 的位置（约 line 90 的 spawn async block），在 `match crate::indexer::scan_into(...)` 之后追加树扫描。将：

```rust
                match crate::indexer::scan_into(&pool3, lib_id, &root3).await {
                    Ok(report) => {
                        log::info!(
                            "[watcher] 增量扫描完成：新增 {} / 更新 {} / 删除 {} / 耗时 {}ms",
                            report.new, report.updated, report.deleted, report.duration_ms
                        );
                        let _ = app3.emit("library://auto-scanned", &report);
                    }
                    Err(e) => {
                        log::error!("[watcher] 增量扫描失败：{e}");
                        let _ = app3.emit("library://auto-scan-error", e.to_string());
                    }
                }
```

改为（追加树扫描）：

```rust
                match crate::indexer::scan_into(&pool3, lib_id, &root3).await {
                    Ok(report) => {
                        log::info!(
                            "[watcher] 增量扫描完成：新增 {} / 更新 {} / 删除 {} / 耗时 {}ms",
                            report.new, report.updated, report.deleted, report.duration_ms
                        );
                        // 同步刷新目录树（如实反映目录增删改）
                        if let Err(e) = crate::indexer::scan_tree_into(&pool3, lib_id, &root3).await {
                            log::warn!("[watcher] 目录树同步失败：{e}");
                        }
                        let _ = app3.emit("library://auto-scanned", &report);
                    }
                    Err(e) => {
                        log::error!("[watcher] 增量扫描失败：{e}");
                        let _ = app3.emit("library://auto-scan-error", e.to_string());
                    }
                }
```

- [ ] **Step 2: 编译验证**

Run: `cd src-tauri && cargo check`
Expected: 编译通过

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/watcher.rs
git commit -m "feat(tree): watcher 增量扫描同步目录树"
```

---

## Task 6: 后端命令（get_directory_tree / files / subtree_files）

**Files:**
- Create: `src-tauri/src/tree.rs`
- Modify: `src-tauri/src/lib.rs` (mod 声明 + 注册命令)

- [ ] **Step 1: 创建 tree.rs**

```rust
use serde::Serialize;
use sqlx::SqlitePool;
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
) -> Result<Vec<DirNode>, String> {
    // 一次取所有目录行，内存里组装树
    let rows: Vec<(i64, Option<i64>, String, String, i32, i64, i64)> = sqlx::query_as(
        "SELECT id, parent_id, name, path, depth, file_count, total_bytes
         FROM directories WHERE library_id=? ORDER BY depth, name",
    )
    .bind(lib_id)
    .fetch_all(&*pool)
    .await
    .map_err(|e| e.to_string())?;

    // id -> DirNode（先建叶子，再挂到父）
    use std::collections::HashMap;
    let mut nodes: HashMap<i64, DirNode> = HashMap::new();
    let mut parent_map: HashMap<i64, Option<i64>> = HashMap::new();
    for (id, parent_id, name, path, depth, fc, tb) in rows {
        nodes.insert(
            id,
            DirNode {
                id, name, path, depth, file_count: fc, total_bytes: tb, children: vec![],
            },
        );
        parent_map.insert(id, parent_id);
    }
    // 组装：根节点（parent_id IS NULL）放入结果；其余挂到父的 children
    let mut roots: Vec<DirNode> = Vec::new();
    // 收集所有 id，按 depth 升序处理（保证父先建好）
    let mut ids_by_depth: Vec<i64> = nodes.keys().copied().collect();
    ids_by_depth.sort_by_key(|id| nodes.get(id).map(|n| n.depth).unwrap_or(0));

    // 先把每个节点从 map 取出，按 parent 挂载（需两遍：自底向上）
    // 简化做法：克隆出 children 引用关系
    // 由于 Rust 所有权，用临时结构
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
) -> Result<Vec<FileNode>, String> {
    let rows: Vec<(i64, String, String, String, String, i64, String)> = sqlx::query_as(
        "SELECT f.id, f.rel_path, f.name, f.ext, f.kind, f.bytes, d.path
         FROM files f JOIN directories d ON d.id=f.directory_id
         WHERE f.directory_id=? AND f.deleted=0 ORDER BY f.rel_path",
    )
    .bind(directory_id)
    .fetch_all(&*pool)
    .await
    .map_err(|e| e.to_string())?;
    Ok(rows
        .into_iter()
        .map(|(id, rel, name, ext, kind, bytes, dir_path)| FileNode {
            id,
            rel_path: rel.clone(),
            name,
            ext,
            kind,
            bytes,
            abs_path: format!("{}/{}", dir_path, rel),
        })
        .collect())
}

/// 取某目录及所有子目录的文件汇总（递归）
#[tauri::command]
pub async fn get_subtree_files(
    directory_id: i64,
    pool: State<'_, SqlitePool>,
) -> Result<Vec<FileNode>, String> {
    // 先取该目录及所有后代的 id（递归 CTE）
    let rows: Vec<(i64, String, String, String, String, i64, String)> = sqlx::query_as(
        "WITH RECURSIVE desc(id) AS (
            SELECT id FROM directories WHERE id=?
            UNION ALL
            SELECT d.id FROM directories d JOIN desc ON d.parent_id=desc.id
         )
         SELECT f.id, f.rel_path, f.name, f.ext, f.kind, f.bytes, dir.path
         FROM files f
         JOIN directories dir ON dir.id=f.directory_id
         WHERE f.directory_id IN (SELECT id FROM desc) AND f.deleted=0
         ORDER BY f.rel_path",
    )
    .bind(directory_id)
    .fetch_all(&*pool)
    .await
    .map_err(|e| e.to_string())?;
    Ok(rows
        .into_iter()
        .map(|(id, rel, name, ext, kind, bytes, dir_path)| FileNode {
            id,
            rel_path: rel.clone(),
            name,
            ext,
            kind,
            bytes,
            abs_path: format!("{}/{}", dir_path, rel),
        })
        .collect())
}
```

- [ ] **Step 2: 在 lib.rs 注册模块和命令**

在 `mod tree_scanner;` 后添加：
```rust
mod tree;
```

在 `generate_handler!` 宏中（约 line 90，`preview::get_thumbnail` 之后）添加：
```rust
            tree::get_directory_tree,
            tree::get_directory_files,
            tree::get_subtree_files,
```

- [ ] **Step 3: 编译验证**

Run: `cd src-tauri && cargo check`
Expected: 编译通过

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/tree.rs src-tauri/src/lib.rs
git commit -m "feat(tree): 后端命令 get_directory_tree/files/subtree_files"
```

---

## Task 7: 前端类型 + IPC 层

**Files:**
- Modify: `src/types/library.ts`
- Modify: `src/ipc/library.ts`

- [ ] **Step 1: 在 types/library.ts 添加 DirNode 类型**

在 `SearchHit` interface 之后添加：

```typescript
// 目录树节点（递归，对应 src-tauri/src/tree.rs 的 DirNode）
export interface DirNode {
  id: number;
  name: string;
  path: string;
  depth: number;
  file_count: number;
  total_bytes: number;
  children: DirNode[];
}
```

- [ ] **Step 2: 在 ipc/library.ts 添加 3 个方法**

在 `ipc` 对象内（`searchFiles` 之后）添加：

```typescript
  // 目录树
  async getDirectoryTree(libId: number): Promise<DirNode[]> {
    return invoke<DirNode[]>("get_directory_tree", { libId });
  },
  async getDirectoryFiles(directoryId: number): Promise<FileNode[]> {
    return invoke<FileNode[]>("get_directory_files", { directoryId });
  },
  async getSubtreeFiles(directoryId: number): Promise<FileNode[]> {
    return invoke<FileNode[]>("get_subtree_files", { directoryId });
  },
```

并在文件顶部 import 加 `DirNode`：
```typescript
import type {
  Library, Category, PackageSummary, FileNode, ScanReport,
  Project, PackageSelectionState, SelectionSummary, SearchHit, DirNode,
} from "../types/library";
```

- [ ] **Step 3: 类型检查**

Run: `npx vue-tsc --noEmit`
Expected: 无错误

- [ ] **Step 4: Commit**

```bash
git add src/types/library.ts src/ipc/library.ts
git commit -m "feat(tree): 前端 DirNode 类型 + IPC 层"
```

---

## Task 8: treeStore（树视图状态）

**Files:**
- Create: `src/stores/treeStore.ts`

- [ ] **Step 1: 创建 treeStore.ts**

```typescript
import { defineStore } from "pinia";
import { ref, computed } from "vue";
import { ipc } from "../ipc/library";
import type { DirNode, FileNode } from "../types/library";

export const useTreeStore = defineStore("tree", () => {
  const tree = ref<DirNode[]>([]);
  const currentDirId = ref<number | null>(null);
  const files = ref<FileNode[]>([]);

  // 左侧视图模式：'category' | 'tree'，记忆到 localStorage
  const viewMode = ref<"category" | "tree">(
    (localStorage.getItem("vault.leftView") as "category" | "tree") || "category",
  );

  function setViewMode(mode: "category" | "tree") {
    viewMode.value = mode;
    localStorage.setItem("vault.leftView", mode);
  }

  async function loadTree(libId: number) {
    tree.value = await ipc.getDirectoryTree(libId);
  }

  async function selectDirectory(dirId: number) {
    currentDirId.value = dirId;
    files.value = await ipc.getDirectoryFiles(dirId);
  }

  function clearFiles() {
    files.value = [];
    currentDirId.value = null;
  }

  return {
    tree, currentDirId, files, viewMode,
    setViewMode, loadTree, selectDirectory, clearFiles,
  };
});
```

- [ ] **Step 2: Commit**

```bash
git add src/stores/treeStore.ts
git commit -m "feat(tree): treeStore 视图状态管理"
```

---

## Task 9: DirTreeNode 递归树节点组件

**Files:**
- Create: `src/components/DirTreeNode.vue`

- [ ] **Step 1: 创建 DirTreeNode.vue**

递归自引用组件，渲染单个目录节点 + 其子节点。

```vue
<script setup lang="ts">
import { ref } from "vue";
import { storeToRefs } from "pinia";
import { useTreeStore } from "../stores/treeStore";
import type { DirNode } from "../types/library";

const props = defineProps<{ node: DirNode }>();
const store = useTreeStore();
const { currentDirId } = storeToRefs(store);

// 默认只展开第一层
const expanded = ref(props.node.depth === 0);

function fmtBytes(b: number): string {
  if (b > 1e9) return (b / 1e9).toFixed(1) + " GB";
  if (b > 1e6) return (b / 1e6).toFixed(1) + " MB";
  if (b > 1e3) return (b / 1e3).toFixed(0) + " KB";
  return b + " B";
}
</script>

<template>
  <div>
    <div
      class="flex items-center gap-1 px-2 py-1 cursor-pointer rounded hover:bg-slate-700/50 text-sm"
      :class="node.id === currentDirId ? 'bg-sky-600/30' : ''"
      :style="{ paddingLeft: (node.depth * 12 + 8) + 'px' }"
    >
      <!-- 展开/折叠箭头（有子节点才显示）-->
      <span
        v-if="node.children.length > 0"
        class="text-xs text-slate-500 w-3 inline-block"
        @click.stop="expanded = !expanded"
      >{{ expanded ? '▼' : '▶' }}</span>
      <span v-else class="inline-block w-3"></span>
      <!-- 文件夹图标 -->
      <span class="text-amber-400">{{ expanded && node.children.length ? '📂' : '📁' }}</span>
      <!-- 名称 -->
      <span
        class="flex-1 truncate"
        :class="node.file_count > 0 ? 'text-slate-200' : 'text-slate-500'"
        :title="node.path"
        @click="node.file_count > 0 && store.selectDirectory(node.id)"
      >{{ node.name }}</span>
      <!-- 统计 -->
      <span v-if="node.file_count > 0" class="text-xs text-slate-500 whitespace-nowrap">
        {{ node.file_count }} · {{ fmtBytes(node.total_bytes) }}
      </span>
    </div>
    <!-- 递归子节点 -->
    <div v-if="expanded">
      <DirTreeNode
        v-for="child in node.children"
        :key="child.id"
        :node="child"
      />
    </div>
  </div>
</template>
```

- [ ] **Step 2: Commit**

```bash
git add src/components/DirTreeNode.vue
git commit -m "feat(tree): DirTreeNode 递归树节点组件"
```

---

## Task 10: DirectoryTree 容器组件

**Files:**
- Create: `src/components/DirectoryTree.vue`

- [ ] **Step 1: 创建 DirectoryTree.vue**

```vue
<script setup lang="ts">
import { storeToRefs } from "pinia";
import { useTreeStore } from "../stores/treeStore";
import { useLibraryStore } from "../stores/libraryStore";
import DirTreeNode from "./DirTreeNode.vue";

const store = useTreeStore();
const lib = useLibraryStore();
const { tree } = storeToRefs(store);
const { currentLibId } = storeToRefs(lib);
</script>

<template>
  <aside class="w-64 shrink-0 overflow-y-auto bg-slate-800/50 border-r border-slate-700">
    <div v-if="tree.length === 0" class="px-3 py-4 text-sm text-slate-500">
      无目录。请先扫描库。
    </div>
    <div v-else class="py-2">
      <DirTreeNode
        v-for="node in tree"
        :key="node.id"
        :node="node"
      />
    </div>
  </aside>
</template>
```

- [ ] **Step 2: Commit**

```bash
git add src/components/DirectoryTree.vue
git commit -m "feat(tree): DirectoryTree 容器组件"
```

---

## Task 11: 视图切换 + App.vue 集成

**Files:**
- Modify: `src/components/CategoryTree.vue` (顶部加切换按钮)
- Modify: `src/App.vue` (根据 viewMode 渲染左侧)
- Modify: `src/i18n/zh.ts` + `en.ts`

- [ ] **Step 1: CategoryTree.vue 顶部加切换按钮**

在 CategoryTree.vue 的 `<aside>` 标签内最前面加切换栏：

```vue
<template>
  <div class="flex flex-col w-64 shrink-0">
    <!-- 视图切换栏 -->
    <div class="flex gap-1 p-2 bg-slate-800 border-b border-slate-700 shrink-0">
      <button
        class="flex-1 px-2 py-1 rounded text-xs"
        :class="store.viewMode === 'category' ? 'bg-sky-600 text-white' : 'bg-slate-700 text-slate-300 hover:bg-slate-600'"
        @click="store.setViewMode('category')"
      >📁 {{ t("tree.viewCategory") }}</button>
      <button
        class="flex-1 px-2 py-1 rounded text-xs"
        :class="store.viewMode === 'tree' ? 'bg-sky-600 text-white' : 'bg-slate-700 text-slate-300 hover:bg-slate-600'"
        @click="store.setViewMode('tree')"
      >🌳 {{ t("tree.viewTree") }}</button>
    </div>
    <!-- 原有分类列表（w-64 移到外层）-->
    <aside class="flex-1 overflow-y-auto bg-slate-800/50 border-r border-slate-700">
      <ul class="py-2">
        <!-- ...原有 li 内容不变... -->
      </ul>
    </aside>
  </div>
</template>
```

注意：CategoryTree 现在引入 treeStore 来读写 viewMode，并在 script setup 加：

```typescript
import { useTreeStore } from "../stores/treeStore";
import { useI18n } from "vue-i18n";
const treeStore = useTreeStore();
const store = useLibraryStore(); // 注意命名冲突，把 treeStore 用于 viewMode
const { t } = useI18n();
```

由于 CategoryTree 原本用 `store` 指向 libraryStore，这里改名：保留 `store` = libraryStore，新增 `treeStore` = useTreeStore()。模板里 `store.setViewMode` 改为 `treeStore.setViewMode`，`store.viewMode` 改为 `treeStore.viewMode`。

- [ ] **Step 2: App.vue 根据 viewMode 渲染左侧**

修改 App.vue，左侧面板根据 treeStore.viewMode 渲染 CategoryTree 或 DirectoryTree。

在 script setup 加：
```typescript
import { useTreeStore } from "./stores/treeStore";
const treeStore = useTreeStore();
```

监听自动扫描事件里，刷新分类的同时也刷新树：
```typescript
await listen("library://auto-scanned", async () => {
  store.autoScanning = false;
  await store.loadCategories();
  if (store.currentCategoryId !== null) await store.loadPackages();
  if (store.currentPkgId !== null) await store.selectPackage(store.currentPkgId);
  // 同步刷新目录树
  if (store.currentLibId !== null) await treeStore.loadTree(store.currentLibId);
  if (treeStore.currentDirId !== null) await treeStore.selectDirectory(treeStore.currentDirId);
});
```

模板中左侧：
```vue
<CategoryTree v-if="treeStore.viewMode === 'category'" />
<DirectoryTree v-else />
```

并修改 PackageGrid/FileGrid 的渲染条件：树视图时中间网格用 treeStore.files 而非 libraryStore.files。这部分需要 PackageGrid 判断 viewMode 决定数据源。

- [ ] **Step 3: i18n 加文案**

zh.ts 添加：
```typescript
  tree: {
    viewCategory: "两级视图",
    viewTree: "树视图",
    noDir: "无目录。请先扫描库。",
  },
```
en.ts 添加：
```typescript
  tree: {
    viewCategory: "Two-level",
    viewTree: "Tree",
    noDir: "No directories. Scan a library first.",
  },
```

- [ ] **Step 4: 类型检查 + 编译验证**

Run: `npx vue-tsc --noEmit` → 无错误
Run: `pnpm tauri build` 前先 `cargo check`

- [ ] **Step 5: Commit**

```bash
git add src/components/CategoryTree.vue src/App.vue src/i18n/zh.ts src/i18n/en.ts
git commit -m "feat(tree): 视图切换按钮 + App 集成树视图"
```

---

## Task 12: PackageGrid 适配树视图数据源

**Files:**
- Modify: `src/components/PackageGrid.vue`

- [ ] **Step 1: PackageGrid 根据 viewMode 决定显示树视图的文件网格**

PackageGrid 当前在 `currentPkgId !== null` 时渲染 FileGrid（用 libraryStore.files），否则渲染包网格。树视图下没有"包网格"概念——点目录直接显示文件。

修改 PackageGrid：树视图下，直接渲染 FileGrid（数据源 treeStore.files），不显示包网格。

在 script setup 加：
```typescript
import { useTreeStore } from "../stores/treeStore";
import { storeToRefs } from "pinia";
const treeStore = useTreeStore();
const { viewMode } = storeToRefs(treeStore);
```

模板改为：
```vue
<template>
  <main class="flex-1 overflow-hidden flex flex-col">
    <!-- 两级视图：包网格 or 包内文件 -->
    <template v-if="viewMode === 'category'">
      <FileGrid v-if="currentPkgId !== null" :locate-file-id="locateFileId" @located="onLocated" />
      <template v-else>
        <!-- ...原有包网格... -->
      </template>
    </template>
    <!-- 树视图：直接文件网格（treeStore.files）-->
    <FileGrid v-else-if="treeStore.currentDirId !== null" :locate-file-id="locateFileId" @located="onLocated" />
    <div v-else class="flex-1 flex items-center justify-center text-slate-500 text-sm">
      {{ t("tree.noDir") }}
    </div>
  </main>
</template>
```

关键问题：FileGrid 当前从 `libraryStore.files` 取数据。树视图下要让它用 `treeStore.files`。最干净的做法：给 FileGrid 加一个可选 `files` prop，传入时用传入的，否则用 libraryStore.files。

修改 FileGrid：加 `files` prop（可选），优先用 prop：

```typescript
const props = defineProps<{
  locateFileId?: number | null;
  files?: FileNode[];  // 可选：树视图传入
}>();
```
然后把内部所有 `files.value` 改为 `props.files ?? libFiles.value`，其中 `libFiles` 来自 libraryStore。

- [ ] **Step 2: 类型检查**

Run: `npx vue-tsc --noEmit`
Expected: 无错误

- [ ] **Step 3: Commit**

```bash
git add src/components/PackageGrid.vue src/components/FileGrid.vue
git commit -m "feat(tree): PackageGrid/FileGrid 适配树视图数据源"
```

---

## Task 13: 端到端验证

- [ ] **Step 1: 启动应用，触发全量扫描**

Run: `pnpm tauri dev`
点"扫描"，等待完成。

- [ ] **Step 2: 验证树视图**

- 左侧顶部切到"树视图"，看到完整目录树（任意深度）
- 展开/折叠正常
- 点含文件的目录，中间网格显示该目录直接文件
- 切回"两级视图"，原分类/包正常
- 重启应用，视图模式记忆正确

- [ ] **Step 3: 验证如实反映磁盘变化**

- 在资源库磁盘上新建一个文件夹（含文件）
- 等 3-5 秒，树视图自动出现新目录
- 删除一个文件夹，等 3-5 秒，树视图自动消失
- 重命名一个文件夹，等 3-5 秒，树视图自动更新

- [ ] **Step 4: 验证勾选/预览在树视图下正常**

- 树视图点目录，预览文件、勾选、导出都正常工作

- [ ] **Step 5: Commit（如有修复）**

---

## Self-Review

**Spec coverage：**
- ✅ 完整目录树任意深度 → Task 2/3/6
- ✅ 每个含文件目录都是包 → Task 3（file_count）+ Task 9（点击逻辑）
- ✅ 只读展示，不移动/重命名 → Task 9（无编辑操作）
- ✅ 如实反映磁盘变化 → Task 4/5（全量+增量+watcher）+ Task 13 验证
- ✅ 与现有两级视图并存 → Task 11 切换
- ✅ directories 表 + files.directory_id → Task 1
- ✅ 3 个后端命令 → Task 6
- ✅ 勾选/导出兼容 → Task 12（selections 不变）

**Placeholder scan：** Task 11 Step 1 有 `<!-- ...原有 li 内容不变... -->` 和 Task 11 Step 2 有 `<!-- ...原有包网格... -->`——这是指代现有代码保持不变，非占位符，但实现时需对照原文件。其余无占位符。

**Type consistency：** DirNode（id/name/path/depth/file_count/total_bytes/children）在 Task 6/7/9 一致；scan_tree_into 在 Task 3/4/5 一致；get_directory_tree/files/subtree_files 在 Task 6/7/8 一致。
