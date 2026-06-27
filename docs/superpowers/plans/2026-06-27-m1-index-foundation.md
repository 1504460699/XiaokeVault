# M1 索引底座 实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 让软件能添加 GameAssets 库、用可配置类型注册表扫描全库、写入 SQLite 索引，并提供查询 API。

**Architecture:** Rust 后端建 4 个模块（scanner/indexer/library/类型注册表）。SQLite 经 sqlx 连接。扫描用 walkdir+rayon 并行。类型注册表内置默认常量 + 数据库覆盖。前端先不做 UI，M1 只验证后端 command 能跑通（用 tauri dev 控制台或临时测试页）。

**Tech Stack:** Rust, sqlx (SQLite), walkdir, rayon, serde, Tauri 2 commands

**Spec:** `docs/superpowers/specs/2026-06-27-game-asset-manager-design.md` §3, §4.1-4.3

---

## 文件结构

```
src-tauri/
├── Cargo.toml                      # 加依赖
├── src/
│   ├── main.rs                     # 不变
│   ├── lib.rs                      # 注册新 command
│   ├── db.rs                       # 连接池 + 建表迁移
│   ├── types.rs                    # 通用领域类型 + serde 序列化
│   ├── asset_types.rs              # 类型注册表：内置默认常量 + CRUD
│   ├── scanner.rs                  # 文件树扫描（walkdir+rayon）
│   ├── indexer.rs                  # SQLite 读写 + 增量算法
│   └── library.rs                  # 库管理 + 来源解析 command
```

---

## Task 1: 添加依赖与建数据库连接

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Create: `src-tauri/src/db.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: 在 Cargo.toml 的 [dependencies] 末尾追加**

```toml
sqlx = { version = "0.8", features = ["runtime-tokio", "sqlite", "macros"] }
tokio = { version = "1", features = ["full"] }
walkdir = "2"
rayon = "1"
chrono = "0.4"
dirs = "5"
```

- [ ] **Step 2: 创建 src-tauri/src/db.rs**

数据库文件放 Tauri app data 目录。用 sqlx 的 Any/SqlitePool。

```rust
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;
use std::path::PathBuf;
use std::str::FromStr;

/// 返回数据库文件路径：<app_data>/com.xiaoke.tauri-app/index.db
pub fn db_path() -> PathBuf {
    let mut p = dirs::data_dir().expect("no data dir");
    p.push("com.xiaoke.tauri-app");
    std::fs::create_dir_all(&p).expect("create app data dir");
    p.push("index.db");
    p
}

/// 创建连接池（启用外键）。
pub async fn connect() -> Result<SqlitePool, sqlx::Error> {
    let path = db_path();
    let opts = SqliteConnectOptions::from_str(&format!("sqlite://{}", path.display()))?
        .create_if_missing(true)
        .foreign_keys(true);
    SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(opts)
        .await
}
```

- [ ] **Step 3: 在 src-tauri/src/lib.rs 顶部加模块声明**

把 lib.rs 改为：

```rust
mod db;

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![greet])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 4: 验证编译**

Run: `cd src-tauri && cargo check`
Expected: 编译通过（首次会下载 crates，可能数分钟）。如有依赖版本冲突，调整版本号。

- [ ] **Step 5: Commit**

```bash
git -C "D:/Xiaoke/Works/Tools/XiaokeTools" init 2>nul & git -C "D:/Xiaoke/Works/Tools/XiaokeTools" add -A
git -C "D:/Xiaoke/Works/Tools/XiaokeTools" commit -m "feat(m1): add db deps and connection pool"
```

> 注：项目尚未 git init，首次需初始化。后续 Task 的 commit 命令不再重复 init。

---

## Task 2: 建表迁移（libraries/categories/packages/files/asset_types）

**Files:**
- Modify: `src-tauri/src/db.rs`
- Create: `src-tauri/migrations/0001_init.sql`
- Modify: `src-tauri/Cargo.toml`（若用 sqlx migrate 则需 build 设置；这里改用手动 SQL 执行，避免 build 复杂度）

- [ ] **Step 1: 创建 src-tauri/migrations/0001_init.sql**

```sql
CREATE TABLE IF NOT EXISTS libraries (
    id           INTEGER PRIMARY KEY,
    name         TEXT NOT NULL,
    root_path    TEXT NOT NULL UNIQUE,
    created_at   INTEGER NOT NULL,
    last_scan_at INTEGER
);

CREATE TABLE IF NOT EXISTS categories (
    id          INTEGER PRIMARY KEY,
    library_id  INTEGER NOT NULL REFERENCES libraries(id) ON DELETE CASCADE,
    name        TEXT NOT NULL,
    sort_order  INTEGER NOT NULL,
    UNIQUE(library_id, name)
);

CREATE TABLE IF NOT EXISTS packages (
    id             INTEGER PRIMARY KEY,
    category_id    INTEGER NOT NULL REFERENCES categories(id) ON DELETE CASCADE,
    name           TEXT NOT NULL,
    path           TEXT NOT NULL,
    file_count     INTEGER DEFAULT 0,
    total_bytes    INTEGER DEFAULT 0,
    has_zip        INTEGER DEFAULT 0,
    source_url     TEXT,
    source_title   TEXT,
    license        TEXT,
    license_source TEXT,
    UNIQUE(category_id, name)
);

CREATE TABLE IF NOT EXISTS files (
    id           INTEGER PRIMARY KEY,
    package_id   INTEGER NOT NULL REFERENCES packages(id) ON DELETE CASCADE,
    rel_path     TEXT NOT NULL,
    name         TEXT NOT NULL,
    ext          TEXT NOT NULL,
    kind         TEXT NOT NULL,
    bytes        INTEGER NOT NULL,
    width        INTEGER,
    height       INTEGER,
    frame_count  INTEGER,
    modified_at  INTEGER NOT NULL,
    content_hash TEXT,
    deleted      INTEGER DEFAULT 0,
    UNIQUE(package_id, rel_path)
);

CREATE TABLE IF NOT EXISTS asset_types (
    kind        TEXT PRIMARY KEY,
    label       TEXT NOT NULL,
    extensions  TEXT NOT NULL,
    viewer      TEXT NOT NULL,
    icon        TEXT,
    is_source   INTEGER DEFAULT 0,
    built_in    INTEGER DEFAULT 0,
    sort_order  INTEGER DEFAULT 0
);
```

（duplicate_groups / selections / projects 表在 M3/M5 引入时再加迁移，M1 不建。）

- [ ] **Step 2: 在 db.rs 末尾加迁移执行函数**

```rust
pub async fn migrate(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    let sql = include_str!("../migrations/0001_init.sql");
    sqlx::query(sql).execute(pool).await?;
    Ok(())
}
```

- [ ] **Step 3: 在 lib.rs 的 run() 里建池并迁移**

```rust
mod db;
use tauri::Manager;

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let pool = tauri::async_runtime::block_on(async {
                let pool = db::connect().await.expect("db connect");
                db::migrate(&pool).await.expect("db migrate");
                pool
            });
            app.manage(pool);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![greet])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 4: 验证编译与运行**

Run: `cd src-tauri && cargo build`
Expected: 编译通过。运行后 `%APPDATA%\com.xiaoke.tauri-app\index.db` 应被创建。

- [ ] **Step 5: Commit**

```bash
git -C "D:/Xiaoke/Works/Tools/XiaokeTools" add -A
git -C "D:/Xiaoke/Works/Tools/XiaokeTools" commit -m "feat(m1): schema migration for libraries/categories/packages/files/types"
```

---

## Task 3: 类型注册表 — 内置默认常量

按设计文档 §3.2 的 13 类内置表。这是扫描时派生 kind 的依据。

**Files:**
- Create: `src-tauri/src/asset_types.rs`

- [ ] **Step 1: 创建 src-tauri/src/asset_types.rs（数据结构 + 内置常量）**

```rust
use serde::{Deserialize, Serialize};

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
        AssetType { kind: "image".into(), label: "图片".into(),
            extensions: vec!["png","jpg","jpeg","webp","bmp","tif","tiff"].into_iter().map(String::from).collect(),
            viewer: "image".into(), icon: Some("image".into()), is_source: false },
        AssetType { kind: "animated".into(), label: "动画".into(),
            extensions: vec!["gif"].into_iter().map(String::from).collect(),
            viewer: "animated".into(), icon: Some("animated".into()), is_source: false },
        AssetType { kind: "vector".into(), label: "矢量".into(),
            extensions: vec!["svg"].into_iter().map(String::from).collect(),
            viewer: "vector".into(), icon: Some("vector".into()), is_source: false },
        AssetType { kind: "audio".into(), label: "音频".into(),
            extensions: vec!["ogg","mp3","wav","flac"].into_iter().map(String::from).collect(),
            viewer: "audio".into(), icon: Some("audio".into()), is_source: false },
        AssetType { kind: "font".into(), label: "字体".into(),
            extensions: vec!["ttf","otf"].into_iter().map(String::from).collect(),
            viewer: "font".into(), icon: Some("font".into()), is_source: false },
        AssetType { kind: "text".into(), label: "文本数据".into(),
            extensions: vec!["txt","xml","json","cs","sh","mat","tmx"].into_iter().map(String::from).collect(),
            viewer: "text".into(), icon: Some("text".into()), is_source: false },
        AssetType { kind: "model3d".into(), label: "3D模型".into(),
            extensions: vec!["obj","mtl","fbx","gltf","glb","dae","dds","tga"].into_iter().map(String::from).collect(),
            viewer: "3d".into(), icon: Some("model3d".into()), is_source: false },
        AssetType { kind: "source_blend".into(), label: "Blender源".into(),
            extensions: vec!["blend"].into_iter().map(String::from).collect(),
            viewer: "binary-source".into(), icon: Some("blend".into()), is_source: true },
        AssetType { kind: "source_pixel".into(), label: "像素源".into(),
            extensions: vec!["ase","xcf"].into_iter().map(String::from).collect(),
            viewer: "binary-source".into(), icon: Some("pixel".into()), is_source: true },
        AssetType { kind: "source_design".into(), label: "设计源".into(),
            extensions: vec!["psd","ai"].into_iter().map(String::from).collect(),
            viewer: "binary-source".into(), icon: Some("design".into()), is_source: true },
        AssetType { kind: "archive".into(), label: "压缩包".into(),
            extensions: vec!["zip","7z","rar"].into_iter().map(String::from).collect(),
            viewer: "fallback".into(), icon: Some("archive".into()), is_source: false },
        AssetType { kind: "legacy_media".into(), label: "旧媒体".into(),
            extensions: vec!["swf"].into_iter().map(String::from).collect(),
            viewer: "fallback".into(), icon: Some("legacy".into()), is_source: false },
        AssetType { kind: "other".into(), label: "其他".into(),
            extensions: vec![],
            viewer: "fallback".into(), icon: Some("file".into()), is_source: false },
    ]
}
```

- [ ] **Step 2: 在 lib.rs 加 `mod asset_types;`**

```rust
mod db;
mod asset_types;
use tauri::Manager;
```

- [ ] **Step 3: 验证编译**

Run: `cd src-tauri && cargo check`
Expected: 通过。

- [ ] **Step 4: Commit**

```bash
git -C "D:/Xiaoke/Works/Tools/XiaokeTools" add -A
git -C "D:/Xiaoke/Works/Tools/XiaokeTools" commit -m "feat(m1): builtin asset type registry (13 kinds)"
```

---

## Task 4: 类型注册表 — 扩展名→kind 查询与合并加载

**Files:**
- Modify: `src-tauri/src/asset_types.rs`

- [ ] **Step 1: 在 asset_types.rs 末尾加 Registry 结构与查询逻辑**

读取时内置默认 + 数据库覆盖合并。扩展名小写后查 kind。

```rust
use sqlx::SqlitePool;

/// 合并后的注册表：扩展名(小写) → AssetType。
pub struct Registry {
    by_ext: std::collections::HashMap<String, AssetType>,
    types: Vec<AssetType>,
}

impl Registry {
    /// 加载：内置默认为底，再用 asset_types 表覆盖/追加。
    pub async fn load(pool: &SqlitePool) -> Result<Self, sqlx::Error> {
        let mut types: Vec<AssetType> = asset_types::builtin_types();
        // 读数据库覆盖/新增项
        let rows: Vec<(String,String,String,String,Option<String>,i64)> = sqlx::query_as(
            "SELECT kind,label,extensions,viewer,icon,is_source FROM asset_types"
        ).fetch_all(pool).await?;
        for (kind,label,exts_json,viewer,icon,is_source) in rows {
            let extensions: Vec<String> = serde_json::from_str(&exts_json).unwrap_or_default();
            let at = AssetType { kind: kind.clone(), label, extensions, viewer, icon, is_source: is_source != 0 };
            if let Some(pos) = types.iter().position(|t| t.kind == kind) {
                types[pos] = at; // 覆盖内置
            } else {
                types.push(at); // 用户新增
            }
        }
        // 建 扩展名→类型 索引
        let mut by_ext = std::collections::HashMap::new();
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

    pub fn all(&self) -> &[AssetType] { &self.types }
}
```

- [ ] **Step 2: 验证编译**

Run: `cd src-tauri && cargo check`
Expected: 通过。

- [ ] **Step 3: Commit**

```bash
git -C "D:/Xiaoke/Works/Tools/XiaokeTools" add -A
git -C "D:/Xiaoke/Works/Tools/XiaokeTools" commit -m "feat(m1): registry load/merge and ext->kind lookup"
```

---

## Task 5: scanner — 文件树扫描

按设计 §4.1。并行遍历，跳过 `_下载脚本/` 与隐藏目录。

**Files:**
- Create: `src-tauri/src/scanner.rs`

- [ ] **Step 1: 创建 src-tauri/src/scanner.rs**

```rust
use rayon::prelude::*;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// 一条扫描结果。
#[derive(Debug, Clone)]
pub struct ScanEntry {
    pub category: String,   // 一级目录名
    pub package: String,    // 二级目录名(包)
    pub rel_path: String,   // 相对包的路径(文件)
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
        .filter(|p| p.is_dir() && !should_skip(p.file_name().and_then(|n| n.to_str()).unwrap_or("")))
        .collect();

    // 每个分类并行扫描
    let entries: Vec<Vec<ScanEntry>> = categories.par_iter().map(|cat_dir| {
        let cat_name = cat_dir.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
        // 该分类下的包(二级目录)
        let packages: Vec<PathBuf> = std::fs::read_dir(cat_dir)
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.is_dir() && !should_skip(p.file_name().and_then(|n| n.to_str()).unwrap_or("")))
            .collect();
        packages.iter().flat_map(|pkg_dir| {
            let pkg_name = pkg_dir.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
            WalkDir::new(pkg_dir).into_iter().filter_map(|e| e.ok()).filter_map(|entry| {
                if !entry.file_type().is_file() { return None; }
                let meta = entry.metadata().ok()?;
                let full = entry.path();
                let rel = full.strip_prefix(pkg_dir).ok()?.to_string_lossy().replace('\\', "/");
                let name = entry.file_name().to_string_lossy().to_string();
                let ext = full.extension().and_then(|e| e.to_str()).map(|s| s.to_lowercase()).unwrap_or_default();
                let modified = meta.modified().ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_secs() as i64).unwrap_or(0);
                Some(ScanEntry {
                    category: cat_name.clone(),
                    package: pkg_name.clone(),
                    rel_path: rel,
                    name,
                    ext,
                    bytes: meta.len(),
                    modified_at: modified,
                })
            }).collect::<Vec<_>>()
        }).collect::<Vec<_>>()
    }).collect();

    entries.into_iter().flatten().collect()
}
```

- [ ] **Step 2: 在 lib.rs 加 `mod scanner;`**

- [ ] **Step 3: 验证编译**

Run: `cd src-tauri && cargo check`
Expected: 通过。

- [ ] **Step 4: Commit**

```bash
git -C "D:/Xiaoke/Works/Tools/XiaokeTools" add -A
git -C "D:/Xiaoke/Works/Tools/XiaokeTools" commit -m "feat(m1): parallel file tree scanner"
```

---

## Task 6: indexer — 全量索引写入

按设计 §4.2 增量算法（全量是增量首次执行：全部 INSERT）。

**Files:**
- Create: `src-tauri/src/indexer.rs`

- [ ] **Step 1: 创建 src-tauri/src/indexer.rs**

```rust
use crate::asset_types::Registry;
use crate::scanner::{self, ScanEntry};
use serde::Serialize;
use sqlx::SqlitePool;
use std::collections::HashSet;
use std::path::Path;

#[derive(Debug, Serialize)]
pub struct ScanReport {
    pub new: u64,
    pub updated: u64,
    pub deleted: u64,
    pub total_files: u64,
    pub duration_ms: u128,
    pub errors: Vec<String>,
    pub unknown_extensions: Vec<(String, u64)>,
}

/// 确保 分类/包 行存在，返回 (category_id, package_id)。
async fn ensure_cat_pkg(
    pool: &SqlitePool, lib_id: i64, category: &str, package: &str, pkg_path: &str,
) -> Result<(i64, i64), sqlx::Error> {
    // 分类
    let sort_order: i64 = category.split('_').next().and_then(|s| s.parse().ok()).unwrap_or(999);
    sqlx::query(
        "INSERT INTO categories(library_id,name,sort_order) VALUES(?,?,?)
         ON CONFLICT(library_id,name) DO NOTHING"
    ).bind(lib_id).bind(category).bind(sort_order).execute(pool).await?;
    let (cat_id,): (i64,) = sqlx::query_as("SELECT id FROM categories WHERE library_id=? AND name=?")
        .bind(lib_id).bind(category).fetch_one(pool).await?;
    // 包
    sqlx::query(
        "INSERT INTO packages(category_id,name,path) VALUES(?,?,?)
         ON CONFLICT(category_id,name) DO UPDATE SET path=excluded.path"
    ).bind(cat_id).bind(package).bind(pkg_path).execute(pool).await?;
    let (pkg_id,): (i64,) = sqlx::query_as("SELECT id FROM packages WHERE category_id=? AND name=?")
        .bind(cat_id).bind(package).fetch_one(pool).await?;
    Ok((cat_id, pkg_id))
}

/// 全量/增量扫描入库。
pub async fn scan_into(
    pool: &SqlitePool, lib_id: i64, root: &Path,
) -> Result<ScanReport, sqlx::Error> {
    let start = std::time::Instant::now();
    let registry = Registry::load(pool).await?;
    let mut unknown: std::collections::HashMap<String,u64> = std::collections::HashMap::new();
    let entries = scanner::scan_library(root);

    let mut new = 0u64; let mut updated = 0u64; let mut deleted = 0u64;
    let mut total_written = 0u64;
    let mut seen: HashSet<(i64,String)> = HashSet::new();

    // 按 (category,package) 分组，减少 ensure 查询
    let mut groups: std::collections::HashMap<(String,String), Vec<ScanEntry>> = std::collections::HashMap::new();
    for e in entries { groups.entry((e.category.clone(), e.package.clone())).or_default().push(e); }

    for ((cat,pkg), files) in groups {
        let pkg_path = root.join(&cat).join(&pkg).to_string_lossy().replace('\\', "/");
        let (cat_id, pkg_id) = ensure_cat_pkg(pool, lib_id, &cat, &pkg, &pkg_path).await?;
        let mut total_bytes = 0i64; let mut file_count = 0i64; let mut has_zip = 0i64;
        for f in &files {
            let kind = if registry.kind_for(&f.ext) == "other" && !f.ext.is_empty() {
                *unknown.entry(f.ext.clone()).or_insert(0) += 1; "other"
            } else { registry.kind_for(&f.ext) };
            // 先查是否已存在，用于区分 new/updated
            let existing: Option<(i64,)> = sqlx::query_as(
                "SELECT id FROM files WHERE package_id=? AND rel_path=?")
                .bind(pkg_id).bind(&f.rel_path).fetch_optional(pool).await?;
            sqlx::query(
                "INSERT INTO files(package_id,rel_path,name,ext,kind,bytes,modified_at,deleted)
                 VALUES(?,?,?,?,?,?,?,0)
                 ON CONFLICT(package_id,rel_path) DO UPDATE SET
                   name=excluded.name, ext=excluded.ext, kind=excluded.kind,
                   bytes=excluded.bytes, modified_at=excluded.modified_at, deleted=0"
            ).bind(pkg_id).bind(&f.rel_path).bind(&f.name).bind(&f.ext)
             .bind(kind).bind(f.bytes as i64).bind(f.modified_at)
             .execute(pool).await?;
            match existing {
                Some(_) => updated += 1,
                None => new += 1,
            }
            total_written += 1;
            seen.insert((pkg_id, f.rel_path.clone()));
            total_bytes += f.bytes as i64; file_count += 1;
            if f.ext == "zip" || f.ext == "7z" || f.ext == "rar" { has_zip = 1; }
        }
        // 标记消失文件为软删除并计数
        let existing_rows: Vec<(i64,String)> = sqlx::query_as(
            "SELECT id, rel_path FROM files WHERE package_id=? AND deleted=0")
            .bind(pkg_id).fetch_all(pool).await?;
        for (id, rp) in existing_rows {
            if !seen.contains(&(pkg_id, rp)) {
                sqlx::query("UPDATE files SET deleted=1 WHERE id=?").bind(id).execute(pool).await?;
                deleted += 1;
            }
        }
        sqlx::query("UPDATE packages SET file_count=?, total_bytes=?, has_zip=? WHERE id=?")
            .bind(file_count).bind(total_bytes).bind(has_zip).bind(pkg_id).execute(pool).await?;
    }

    Ok(ScanReport {
        new, updated, deleted,
        total_files: total_written,
        duration_ms: start.elapsed().as_millis(),
        errors: vec![],
        unknown_extensions: unknown.into_iter().collect(),
    })
}
```

> **实现说明（写给执行者）**：上面 new/updated/deleted 的精确计数逻辑较繁琐。执行时可简化为：new = 本次扫描写入的总文件数，updated/deleted 暂置 0 或粗略统计。M1 验收重点是文件数正确和性能，精确计数在 M3 再打磨。若你倾向严谨，可在 INSERT 前先 SELECT 判断存在性来区分 new/updated， vanished 数即 deleted。

- [ ] **Step 2: 在 lib.rs 加 `mod indexer;`**

- [ ] **Step 3: 验证编译**

Run: `cd src-tauri && cargo check`
Expected: 通过。

- [ ] **Step 4: Commit**

```bash
git -C "D:/Xiaoke/Works/Tools/XiaokeTools" add -A
git -C "D:/Xiaoke/Works/Tools/XiaokeTools" commit -m "feat(m1): full scan indexing into sqlite"
```

---

## Task 7: library + indexer command 注册

把 add_library / scan_library_full / get_categories / get_packages / get_package_files 暴露为 Tauri command。

**Files:**
- Create: `src-tauri/src/library.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: 创建 src-tauri/src/library.rs**

```rust
use crate::db;
use crate::indexer::{self, ScanReport};
use serde::Serialize;
use sqlx::SqlitePool;
use std::path::PathBuf;
use tauri::State;

#[derive(Debug, Serialize)]
pub struct Library { pub id: i64, pub name: String, pub root_path: String }

#[derive(Debug, Serialize)]
pub struct Category { pub id: i64, pub name: String, pub sort_order: i64,
    pub package_count: i64, pub file_count: i64, pub total_bytes: i64 }

#[derive(Debug, Serialize)]
pub struct PackageSummary { pub id: i64, pub name: String, pub path: String,
    pub file_count: i64, pub total_bytes: i64, pub has_zip: bool, pub license: Option<String> }

#[derive(Debug, Serialize)]
pub struct FileNode { pub id: i64, pub rel_path: String, pub name: String,
    pub ext: String, pub kind: String, pub bytes: i64 }

#[tauri::command]
pub async fn add_library(name: String, root_path: String, pool: State<'_, SqlitePool>)
    -> Result<Library, String> {
    let now = chrono::Utc::now().timestamp();
    sqlx::query("INSERT INTO libraries(name,root_path,created_at) VALUES(?,?,?)")
        .bind(&name).bind(&root_path).bind(now).execute(&**pool).await
        .map_err(|e| e.to_string())?;
    let (id,): (i64,) = sqlx::query_as("SELECT id FROM libraries WHERE root_path=?")
        .bind(&root_path).fetch_one(&**pool).await.map_err(|e| e.to_string())?;
    Ok(Library { id, name, root_path })
}

#[tauri::command]
pub async fn list_libraries(pool: State<'_, SqlitePool>) -> Result<Vec<Library>, String> {
    let rows: Vec<(i64,String,String)> = sqlx::query_as(
        "SELECT id,name,root_path FROM libraries ORDER BY id")
        .fetch_all(&**pool).await.map_err(|e| e.to_string())?;
    Ok(rows.into_iter().map(|(id,name,root_path)| Library{id,name,root_path}).collect())
}

#[tauri::command]
pub async fn scan_library_full(lib_id: i64, pool: State<'_, SqlitePool>)
    -> Result<ScanReport, String> {
    let (root,): (String,) = sqlx::query_as("SELECT root_path FROM libraries WHERE id=?")
        .bind(lib_id).fetch_one(&**pool).await.map_err(|e| e.to_string())?;
    let report = indexer::scan_into(&pool, lib_id, &PathBuf::from(&root))
        .await.map_err(|e| e.to_string())?;
    let now = chrono::Utc::now().timestamp();
    sqlx::query("UPDATE libraries SET last_scan_at=? WHERE id=?")
        .bind(now).bind(lib_id).execute(&**pool).await.map_err(|e| e.to_string())?;
    Ok(report)
}

#[tauri::command]
pub async fn get_categories(lib_id: i64, pool: State<'_, SqlitePool>)
    -> Result<Vec<Category>, String> {
    let rows: Vec<(i64,String,i64,i64,i64,i64)> = sqlx::query_as(
        "SELECT c.id,c.name,c.sort_order,
           (SELECT COUNT(*) FROM packages p WHERE p.category_id=c.id),
           (SELECT COUNT(*) FROM files f JOIN packages p ON p.id=f.package_id WHERE p.category_id=c.id AND f.deleted=0),
           (SELECT COALESCE(SUM(f.bytes),0) FROM files f JOIN packages p ON p.id=f.package_id WHERE p.category_id=c.id AND f.deleted=0)
         FROM categories c WHERE c.library_id=? ORDER BY c.sort_order")
        .bind(lib_id).fetch_all(&**pool).await.map_err(|e| e.to_string())?;
    Ok(rows.into_iter().map(|(id,name,sort,pc,fc,tb)| Category{
        id,name,sort_order:sort,package_count:pc,file_count:fc,total_bytes:tb}).collect())
}

#[tauri::command]
pub async fn get_packages(category_id: i64, pool: State<'_, SqlitePool>)
    -> Result<Vec<PackageSummary>, String> {
    let rows: Vec<(i64,String,String,i64,i64,i64,Option<String>)> = sqlx::query_as(
        "SELECT id,name,path,file_count,total_bytes,has_zip,license FROM packages
         WHERE category_id=? ORDER BY name")
        .bind(category_id).fetch_all(&**pool).await.map_err(|e| e.to_string())?;
    Ok(rows.into_iter().map(|(id,name,path,fc,tb,hz,lic)| PackageSummary{
        id,name,path,file_count:fc,total_bytes:tb,has_zip:hz!=0,license:lic}).collect())
}

#[tauri::command]
pub async fn get_package_files(pkg_id: i64, pool: State<'_, SqlitePool>)
    -> Result<Vec<FileNode>, String> {
    let rows: Vec<(i64,String,String,String,String,i64)> = sqlx::query_as(
        "SELECT id,rel_path,name,ext,kind,bytes FROM files WHERE package_id=? AND deleted=0 ORDER BY rel_path")
        .bind(pkg_id).fetch_all(&**pool).await.map_err(|e| e.to_string())?;
    Ok(rows.into_iter().map(|(id,rel,name,ext,kind,bytes)| FileNode{
        id,rel_path:rel,name,ext,kind,bytes}).collect())
}
```

- [ ] **Step 2: 更新 lib.rs，注册模块与 command**

```rust
mod db;
mod asset_types;
mod scanner;
mod indexer;
mod library;
use tauri::Manager;

#[tauri::command]
fn greet(name: &str) -> String { format!("Hello, {}!", name) }

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let pool = tauri::async_runtime::block_on(async {
                let pool = db::connect().await.expect("db connect");
                db::migrate(&pool).await.expect("db migrate");
                pool
            });
            app.manage(pool);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            library::add_library,
            library::list_libraries,
            library::scan_library_full,
            library::get_categories,
            library::get_packages,
            library::get_package_files
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 3: 验证编译**

Run: `cd src-tauri && cargo build`
Expected: 通过。

- [ ] **Step 4: Commit**

```bash
git -C "D:/Xiaoke/Works/Tools/XiaokeTools" add -A
git -C "D:/Xiaoke/Works/Tools/XiaokeTools" commit -m "feat(m1): library & indexer tauri commands"
```

---

## Task 8: M1 端到端验收

通过 tauri dev 启动，用前端控制台验证全流程跑通。

- [ ] **Step 1: 删除旧库（若 Task 2 已建过 db）以测干净首次**

```bash
rm -f "$APPDATA/com.xiaoke.tauri-app/index.db" 2>/dev/null
rm -f "$LOCALAPPDATA/com.xiaoke.tauri-app/index.db" 2>/dev/null
```

> Windows 路径：`%APPDATA%\com.xiaoke.tauri-app\index.db`

- [ ] **Step 2: 启动应用**

Run: `npm run tauri dev`
Expected: 应用窗口打开，无 panic。

- [ ] **Step 3: 在应用窗口按 F12 打开 DevTools 控制台，依次执行验证**

打开控制台后输入：

```js
// 1. 添加库
const lib = await window.__TAURI__.core.invoke('add_library', {
  name: 'GameAssets', root_path: 'D:\\Xiaoke\\GameAssets' });
console.log('library:', lib);

// 2. 全量扫描（预计 1 分钟内）
const report = await window.__TAURI__.core.invoke('scan_library_full', { libId: lib.id });
console.log('scan report:', report);

// 3. 查分类
const cats = await window.__TAURI__.core.invoke('get_categories', { libId: lib.id });
console.log('categories:', cats);

// 4. 取第一个分类的包
const pkgs = await window.__TAURI__.core.invoke('get_packages', { categoryId: cats[0].id });
console.log('packages:', pkgs);

// 5. 取第一个包的文件
const files = await window.__TAURI__.core.invoke('get_package_files', { pkgId: pkgs[0].id });
console.log('files (前5):', files.slice(0,5));
```

- [ ] **Step 4: 核对验收标准**

Expected（M1 验收点）：
- `report.total_files` 接近 **42,503**（允许差异，因跳过 _下载脚本）
- `report.duration_ms` < 60000（1 分钟内）
- `cats` 长度约 **13**
- `report.unknown_extensions` 应为空或极少（内置表已覆盖全部格式）
- ogg/ttf/tmx/ase 文件的 `kind` 不应是 `other`（在 files 里抽查）

- [ ] **Step 5: 二次启动验证秒开**

关闭应用，删除 db **不**删，重新 `npm run tauri dev`。
重新执行上面的 get_categories（不扫描），Expected: 秒返（读库即可）。

- [ ] **Step 6: Commit 验收记录**

```bash
git -C "D:/Xiaoke/Works/Tools/XiaokeTools" add -A
git -C "D:/Xiaoke/Works/Tools/XiaokeTools" commit -m "test(m1): e2e acceptance scan of GameAssets (42k files)"
```

---

## M1 完成定义

- [ ] 6 张表创建成功（libraries/categories/packages/files/asset_types + 内置类型）
- [ ] 全量扫描 GameAssets 入库，文件数 ≈ 4.2 万，耗时 < 1 分钟
- [ ] get_categories / get_packages / get_package_files 返回正确数据
- [ ] 内置类型表覆盖库内全部格式（ogg/ttf/tmx/ase 非 other）
- [ ] 二次启动读库秒开
```
