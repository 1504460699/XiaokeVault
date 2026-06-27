# 游戏素材管理软件 设计文档

> 日期: 2026-06-27
> 项目: XiaokeTools (Tauri 2 + Vue 3 + Vite + Rust)
> 目标: 为 `D:\Xiaoke\GameAssets` 下的游戏素材库开发一个桌面管理软件

---

## 1. 背景与需求

### 1.1 现状

用户在 `D:\Xiaoke\GameAssets` 沉淀了一个游戏素材库：

- **规模**: 2.0 GB / 42,503 个文件 / 115 个素材包
- **结构**: 13 个一级分类目录（数字前缀排序），分类下是素材包，包内是文件
- **格式**: 2D（png/jpg/gif/svg）+ 3D（blend/obj/fbx/dae）+ 源文件（psd/xcf）
- **元数据**: 全局有结构化 README；每个包可能含 `_来源.txt`（记录 URL/标题/协议/下载时间）
- **已知痛点**:
  - `_来源.txt` 覆盖率仅 22/115（<20%），版权信息不完整
  - 13 个压缩包与 15 个解压目录并存（重复占用）
  - 02_地牢RPG 类有明显的备份目录重复
  - 文件量极大（02 类单类 2.8 万文件），手动浏览不现实

### 1.2 核心定位

**项目打包导出**：把素材库当"仓库"，软件是"选货+打包"工具，帮助游戏项目从素材库中挑选素材并导出。

（浏览预览、去重整理都是服务于这个核心定位的能力，不是独立产品。）

### 1.3 关键需求决策

| 维度 | 决策 | 依据 |
|---|---|---|
| 挑选粒度 | 混合：勾整包 + 下钻勾单个文件 | 实际工作流既要整包也要精确到文件 |
| 导出产出 | 干净文件夹结构 + 版权署名清单 + 素材索引 manifest + zip 压缩包 | 多种交付形式适配不同场景 |
| 大库策略 | SQLite 索引数据库，首次全扫、增量更新 | 4.2 万文件实时扫描不可行 |
| 预览能力 | 静态缩略图 + GIF/动画帧 + SVG + 3D 模型 + 音频/字体/文本 | 全面覆盖库内格式 |
| 扩展性 | 可配置类型注册表 + 通用预览器 | 资源量和类型都会持续扩充，新类型改配置不改代码 |
| 范围 | 完整版（含去重、3D 预览、多项目管理、类型管理） | 一步到位 |

---

## 2. 整体架构

```
┌─────────────────────────────────────────────────────────┐
│                    Tauri 窗口 (WebView2)                 │
│  ┌───────────────────────────────────────────────────┐  │
│  │  Vue 3 前端 (UI 层，只管展示与交互)                  │  │
│  │  · 分类/包/文件三栏浏览                            │  │
│  │  · 缩略图墙 / 预览器 / 勾选状态                     │  │
│  │  · 导出任务面板                                    │  │
│  │  通过 Tauri invoke ↔ 后端                           │  │
│  └──────────────────────┬────────────────────────────┘  │
│                         │ tauri::command (IPC)           │
│  ┌──────────────────────┴────────────────────────────┐  │
│  │             Rust 后端 (业务核心)                    │  │
│  │  ┌──────────┐ ┌──────────┐ ┌──────────┐            │  │
│  │  │ scanner  │ │ indexer  │ │ exporter │            │  │
│  │  │ 文件树扫描│ │ 索引读写  │ │ 复制打包  │            │  │
│  │  └────┬─────┘ └────┬─────┘ └────┬─────┘            │  │
│  │  ┌────┴────────────┴────────────┴─────┐            │  │
│  │  │           其他模块                   │            │  │
│  │  │  library  │ preview  │ dedup       │            │  │
│  │  │  (库管理) │ (预览)   │ (去重)       │            │  │
│  │  └────────────────────────────────────┘            │  │
│  │         │                          │                │  │
│  │    ┌────▼─────┐              ┌─────▼──────┐         │  │
│  │    │ SQLite   │              │ 文件系统     │         │  │
│  │    │ (索引库)  │              │ GameAssets  │         │  │
│  │    └──────────┘              └────────────┘         │  │
│  └───────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
```

### 2.1 技术栈选型

- **应用框架**: Tauri 2（已搭好脚手架）
- **前端**: Vue 3 + TypeScript + Vite + Tailwind CSS
- **状态管理**: Pinia
- **虚拟滚动**: @tanstack/vue-virtual
- **3D 渲染**: three
- **后端**: Rust
- **数据库**: SQLite（Rust 侧 sqlx，编译期 SQL 校验）

### 2.2 模块职责

| 模块 | 职责 | 依赖 |
|---|---|---|
| `scanner` | 遍历目录树，产出文件元数据流。无状态。 | walkdir, rayon |
| `indexer` | SQLite 读写，全量/增量索引，查询 | scanner, sqlx |
| `library` | 素材库/分类/包的领域逻辑与来源信息管理 | indexer |
| `preview` | 生成/读取缩略图，解析 GIF 帧、SVG、3D 转换 | indexer, fs |
| `dedup` | 检测重复（哈希、压缩包-解压配对、备份目录），软删除 | indexer |
| `exporter` | 按勾选复制文件 + zip 打包 + 生成 credits/manifest | indexer, fs |

### 2.3 IPC 边界

前端只通过 `invoke` 调用 Rust command，不直接碰文件系统。command 是粗粒度业务操作（如 `scan_library`、`export_selection`、`get_package_files`）。长任务通过 Tauri 事件流推送进度。

---

## 3. 数据模型（SQLite 索引库）

数据库文件位置：Tauri app data 目录（`%APPDATA%\com.xiaoke.tauri-app\index.db`），不污染素材库本身。

### 3.1 表结构

```sql
-- 1. 素材库（可管理多个根目录）
CREATE TABLE libraries (
    id          INTEGER PRIMARY KEY,
    name        TEXT NOT NULL,            -- "我的游戏素材库"
    root_path   TEXT NOT NULL UNIQUE,     -- D:\Xiaoke\GameAssets
    created_at  INTEGER NOT NULL,
    last_scan_at INTEGER
);

-- 2. 分类（一级目录，如 01_平台跳跃）
CREATE TABLE categories (
    id          INTEGER PRIMARY KEY,
    library_id  INTEGER NOT NULL REFERENCES libraries(id) ON DELETE CASCADE,
    name        TEXT NOT NULL,            -- "01_平台跳跃"
    sort_order  INTEGER NOT NULL,         -- 按目录名数字前缀排序
    UNIQUE(library_id, name)
);

-- 3. 素材包（二级目录，如 Platformer_Art_Complete_Pack）
CREATE TABLE packages (
    id          INTEGER PRIMARY KEY,
    category_id INTEGER NOT NULL REFERENCES categories(id) ON DELETE CASCADE,
    name        TEXT NOT NULL,            -- 目录名
    path        TEXT NOT NULL,            -- 完整路径
    file_count  INTEGER DEFAULT 0,
    total_bytes INTEGER DEFAULT 0,
    has_zip     INTEGER DEFAULT 0,        -- 1=含同名压缩包(去重信号)
    -- 来源/版权信息（_来源.txt 解析入此，缺失则 NULL，可手动补录）
    source_url     TEXT,
    source_title   TEXT,
    license        TEXT,                  -- CC0 / CC-BY / CC-BY-SA / PUBLIC DOMAIN
    license_source TEXT,                  -- "auto"(自动抓取) / "manual"(手动补录)
    UNIQUE(category_id, name)
);

-- 4. 文件（叶子节点，4.2万行的核心表）
CREATE TABLE files (
    id          INTEGER PRIMARY KEY,
    package_id  INTEGER NOT NULL REFERENCES packages(id) ON DELETE CASCADE,
    rel_path    TEXT NOT NULL,            -- 相对包的路径
    name        TEXT NOT NULL,
    ext         TEXT NOT NULL,            -- png/gif/svg/blend/obj/...
    kind        TEXT NOT NULL,            -- 见 3.2 分类
    bytes       INTEGER NOT NULL,
    width       INTEGER,                  -- 图片才有
    height      INTEGER,
    frame_count INTEGER,                  -- GIF/序列帧才有
    modified_at INTEGER NOT NULL,
    content_hash TEXT,                    -- 仅去重时按需计算
    deleted     INTEGER DEFAULT 0,        -- 软删除标记
    UNIQUE(package_id, rel_path)
);

-- 5. 重复组（去重检测结果）
CREATE TABLE duplicate_groups (
    id          INTEGER PRIMARY KEY,
    reason      TEXT NOT NULL,            -- hash / zip_extracted / likely_backup
    hash        TEXT,                     -- reason=hash 时填
    created_at  INTEGER NOT NULL
);
CREATE TABLE duplicate_members (
    group_id    INTEGER NOT NULL REFERENCES duplicate_groups(id) ON DELETE CASCADE,
    file_id     INTEGER REFERENCES files(id) ON DELETE CASCADE,
    package_id  INTEGER REFERENCES packages(id) ON DELETE CASCADE,
    PRIMARY KEY (group_id, file_id, package_id)
);

-- 6. 勾选状态（导出挑选，断电不丢）
CREATE TABLE projects (
    id          INTEGER PRIMARY KEY,
    name        TEXT NOT NULL,
    export_root TEXT NOT NULL,
    created_at  INTEGER NOT NULL
);
CREATE TABLE selections (
    id          INTEGER PRIMARY KEY,
    scope       TEXT NOT NULL CHECK(scope IN ('package','file','exclude')),
    package_id  INTEGER REFERENCES packages(id) ON DELETE CASCADE,
    file_id     INTEGER REFERENCES files(id) ON DELETE CASCADE,
    project_id  INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    CHECK((scope='package' AND package_id IS NOT NULL) OR
          (scope IN ('file','exclude') AND file_id IS NOT NULL))
);
```

### 3.2 可配置类型注册表（替代硬编码 kind）

> **扩展性约束**：资源量和资源类型都会持续扩充，新类型必须靠"改配置"而非"改代码"支持。

类型定义从代码里抽出，变为**可配置数据**。分为两层：

1. **内置默认表**（随软件发布，编译进二进制）：覆盖库内现有全部格式 + 常见游戏资源格式
2. **数据库覆盖表**：用户在设置 UI 里的修改存这里，覆盖默认值

```sql
-- 类型注册表（仅存用户覆盖/新增项，读取时与内置默认合并）
CREATE TABLE asset_types (
    kind        TEXT PRIMARY KEY,          -- audio / image / model3d ...
    label       TEXT NOT NULL,             -- "音频"（显示名）
    extensions  TEXT NOT NULL,             -- JSON 数组: ["ogg","mp3","wav"]
    viewer      TEXT NOT NULL,             -- 映射到通用预览器，见 3.3
    icon        TEXT,                      -- 图标标识
    is_source   INTEGER DEFAULT 0,         -- 1=源文件（引导用专业软件）
    built_in    INTEGER DEFAULT 0,         -- 1=覆盖内置默认项；0=用户新增
    sort_order  INTEGER DEFAULT 0
);
```

**files 表的 `kind` 字段**改由注册表派生：扫描时按扩展名查注册表得到 kind；查不到则归 `other`，并在扫描报告中统计未知扩展名。

**内置默认类型表**（基于实测库内扩展名，开箱即用）：

| kind | label | 扩展名 | 预览器 | 备注 |
|---|---|---|---|---|
| `image` | 图片 | png jpg jpeg webp bmp tif | image | 含 dds/tga 见 model3d 纹理说明 |
| `animated` | 动画 | gif | animated | |
| `vector` | 矢量 | svg | vector | |
| `audio` | 音频 | ogg mp3 wav flac | audio | |
| `font` | 字体 | ttf otf | font | |
| `text` | 文本数据 | txt xml json cs sh mat tmx | text | tmx=Tiled 地图 |
| `model3d` | 3D 模型 | obj mtl fbx gltf glb dae dds tga | 3d | dds/tga 作 3D 贴图纹理 |
| `source_blend` | Blender 源 | blend | binary-source | 调用本机 Blender 转 |
| `source_pixel` | 像素源 | ase xcf | binary-source | Aseprite/GIMP 源文件 |
| `source_design` | 设计源 | psd ai | binary-source | PS/Illustrator 源文件 |
| `archive` | 压缩包 | zip 7z rar | fallback | 仅作去重信号 |
| `legacy_media` | 旧媒体 | swf | fallback | Flash，仅信息展示 |
| `other` | 其他 | （兜底） | fallback | 注册表未命中的扩展名 |

**未知类型处理**：扫描时未知扩展名归入 `other`，ScanReport 统计 `unknown_extensions[]`。设置页提示"发现 N 个未知类型，是否添加为自定义类型"，引导用户补录。

**用户自助管理**（设置页 UI）：新增 kind、编辑扩展名绑定、选择预览器、标记是否源文件。修改后存 `asset_types` 表，下次扫描对新文件生效（已索引文件可触发"按新注册表重新分类"）。

### 3.3 通用预览器（替代按 kind 写专用组件）

预览器收敛为一组**固定的通用渲染器**，新资源类型只需映射到其中之一，无需新增组件：

| viewer | 覆盖范围 | 实现要点 |
|---|---|---|
| `image` | png/jpg/webp/bmp/tif/dds/tga | 缩略图 + 棋盘格透明背景 |
| `animated` | gif + 序列帧目录 | 缩略图静止，点开逐帧播放 |
| `vector` | svg | 直接内联渲染 |
| `audio` | ogg/mp3/wav/flac | 波形缩略图 + 播放控件 |
| `font` | ttf/otf | 渲染样例字形表 |
| `text` | txt/xml/json/tmx 等 | 语法高亮只读查看 |
| `3d` | obj/fbx/gltf/glb/dae | Three.js 视口 |
| `binary-source` | blend/psd/xcf/ase/ai | 占位 + 文件信息 + "需 XX 打开" |
| `fallback` | other/archive/legacy | 图标 + 文件名/大小 |

预览器数量稳定且有限（9 个），资源类型可无限扩展。前端按 `viewer` 字段路由组件，与 `kind` 解耦。

### 3.4 增量索引策略

首次全扫建库；之后基于 `modified_at` + 目录变化做增量：

- 新增文件 → 插入
- `modified_at` 变了 → 更新（必要时重算 hash）
- 消失文件 → 标记 `deleted=1`（软删除，保留以维持历史勾选关联）

---

## 4. 后端模块详细设计（Rust）

### 4.1 `scanner` — 文件树扫描

**职责**：遍历一个素材库根目录，产出原始文件元数据流。无状态、纯函数式。

**关键决策**：
- 跳过 `_下载脚本/`（README 已标"可忽略"）和隐藏目录
- 识别"素材包"：分类目录（一级）的直接子目录即为包
- 大量小文件的目录（如 02 类 2.8 万文件）用并行遍历（rayon）

**对外接口**（被 indexer 调用，不直接对前端）：

```rust
pub struct ScanEntry { path, name, ext, bytes, modified_at, depth }
pub fn scan_library(root: &Path) -> impl Iterator<Item=ScanEntry>
```

**依赖**：walkdir, rayon

### 4.2 `indexer` — 索引读写核心

**职责**：把 scanner 的产出写入 SQLite，支持全量重建与增量更新；提供查询 API。这是其他模块的底座。

**对外 command**：

```rust
#[tauri::command] async fn scan_library_full(lib_id) -> ScanReport
#[tauri::command] async fn scan_library_incremental(lib_id) -> ScanReport
#[tauri::command] async fn get_categories(lib_id) -> Vec<Category>
#[tauri::command] async fn get_packages(category_id, page) -> Vec<PackageSummary>
#[tauri::command] async fn get_package_files(pkg_id) -> Vec<FileNode>
#[tauri::command] async fn search_files(query, filters) -> Vec<SearchHit>
```

**ScanReport**：`{ new, updated, deleted, total_files, duration_ms, errors[], unknown_extensions[] }`

`unknown_extensions[]` 记录本次扫描中注册表未命中的扩展名及其文件数，供设置页提示用户补录类型。

**增量算法**：

```
对每个 ScanEntry:
  查 files 表 (package_id + rel_path)
  不存在        → INSERT (new++)，按注册表派生 kind
  modified_at 变 → UPDATE (updated++)，必要时重算 kind
  存在且未变    → skip
  扩展名在注册表未命中 → kind='other'，记入 unknown_extensions
遍历库里存在但本次未扫到的 → 标 deleted
```

**依赖**：sqlx（编译期 SQL 校验，类型安全）

### 4.3 `library` — 库与来源管理

**职责**：管理多个素材库；解析/补录版权来源信息；管理类型注册表。

**对外 command**：

```rust
#[tauri::command] fn add_library(name, root_path) -> Library
#[tauri::command] fn list_libraries() -> Vec<Library>
#[tauri::command] fn remove_library(id)
// 版权补录（_来源.txt 覆盖率仅22/115，需手动）
#[tauri::command] fn update_package_source(pkg_id, url, title, license)
#[tauri::command] fn auto_detect_sources() -> DetectedCount  // 批量解析所有 _来源.txt
// 类型注册表管理（支持资源类型持续扩充）
#[tauri::command] fn list_asset_types() -> Vec<AssetType>     // 内置默认 + 数据库覆盖合并
#[tauri::command] fn upsert_asset_type(kind, label, exts, viewer, is_source) // 新增/编辑
#[tauri::command] fn delete_asset_type(kind)                  // 仅删用户新增项
#[tauri::command] async fn reclassify_all(lib_id)             // 按新注册表重新分类全库
```

**`_来源.txt` 解析**（按实测格式）：

```
来源: https://opengameart.org/...
标题: Platformer Art Complete Pack
协议: CC0
下载时间: 2026-06-27 08:29:03
```

正则提取四字段，`license_source='auto'`。手动补录时设 `license_source='manual'`。

**类型注册表合并逻辑**：读取时先加载内置默认表（编译进二进制的常量），再用 `asset_types` 表的记录按 `kind` 覆盖/追加。内置项不可删除，只能被覆盖（`built_in=1` 标记覆盖关系）；用户新增项可删（`built_in=0`）。

**重新分类**：用户修改注册表后，可调用 `reclassify_all` 扫描全库文件按新注册表重算 `kind`。耗时操作，带进度事件。

### 4.4 `preview` — 预览生成

**职责**：为前端提供可显示的预览数据。按**通用预览器（viewer）**分流（见 3.3），与 kind 解耦，使新类型无需改预览代码。

**对外 command**：

```rust
#[tauri::command] async fn get_thumbnail(file_id, size) -> ThumbPath
#[tauri::command] async fn get_gif_frames(file_id) -> GifInfo
#[tauri::command] async fn get_model_glb(file_id) -> ModelPath
#[tauri::command] async fn get_audio_waveform(file_id) -> WaveformInfo  // 新增
```

前端根据文件的 `viewer` 字段决定调用哪个 command；未匹配的 viewer 走 fallback（仅展示文件信息，无 command 调用）。

**缩略图缓存**：首次生成存到 app data（`%APPDATA%\...\thumbs\`），以 `file_id` 命名。命中缓存秒返。缓存设上限（如 2GB），LRU 清理。

**各 viewer 关键算法**：

- `image`（png/jpg/webp/bmp/tif/dds/tga）→ `image` crate 解码 + 缩放，棋盘格透明背景
- `animated`（gif/序列帧）→ `gif` crate 拆帧，存首帧缩略图 + 全帧序列（懒加载）
- `vector`（svg）→ 直接交前端渲染，不生成缩略图
- `audio`（ogg/mp3/wav/flac）→ 解码提取波形峰值，生成波形缩略图；播放交前端 `<audio>`
- `font`（ttf/otf）→ 读取字形表，生成样例字图缩略图
- `text`（txt/xml/json/tmx 等）→ 不生成缩略图，前端直接读内容（限大小，过大只读首部）
- `3d`（obj/fbx/gltf/glb/dae）→ 转 GLB；`.blend` 调用本机 Blender（命令行 `--background` 导出 glb），结果缓存
- `binary-source`（blend/psd/xcf/ase/ai）→ 不预览，返回文件信息供占位展示
- `fallback` → 无 command，前端用图标占位

**依赖**：image, gif, 音频解码见 6.1 风险

### 4.5 `dedup` — 去重检测

**职责**：检测三种重复，写入 `duplicate_groups`。

**对外 command**：

```rust
#[tauri::command] async fn run_dedup(lib_id) -> DedupReport
#[tauri::command] fn get_duplicate_groups(reason?) -> Vec<DupGroup>
#[tauri::command] async fn resolve_duplicate(group_id, action, keep_target)
```

**三种检测**：

1. **`zip_extracted`**（确定性高）：包内同时存在 `xxx.zip` 和 `[解压]_xxx` 目录 → 提示删除 zip
2. **`likely_backup`**（确定性中）：同名/近名包跨分类或同分类共存，README 标"备份"。用名字相似度 + 文件数比对
3. **`hash`**（确定性最高但最贵）：对疑似重复按需算 content_hash，相同即重复。不全局算哈希（4.2万文件太贵），仅在前两种触发后复核

**安全原则**：`resolve_duplicate` 删除前先把目标移到 trash 目录（`%APPDATA%\...\trash\`），软删除可恢复。不直接物理删除。

### 4.6 `exporter` — 导出打包

**职责**：按勾选把文件复制到目标目录或打包成 zip，生成 credits + manifest。

**对外 command**：

```rust
#[tauri::command] async fn create_project(name, export_root) -> Project
#[tauri::command] fn get_selections(project_id) -> SelectionState
#[tauri::command] async fn set_selection(project_id, scope, target_id, checked)
#[tauri::command] async fn run_export(project_id, format, options) -> ExportHandle
#[tauri::command] fn cancel_export(job_id)
```

**导出产出**：

```
<export_root>/<project_name>/              ← 文件夹模式
或 <export_root>/<project_name>.zip        ← zip 模式，内含同样结构
├── assets/                                ← 干净文件夹结构
│   ├── 01_平台跳跃/
│   │   └── Platformer_Art_Complete_Pack/ ...复制选中文件
│   └── 04_图标UI/ ...
├── CREDITS.txt                            ← 版权署名清单（人读）
├── CREDITS.json                           ← 结构化版权（机读）
└── manifest.json                          ← 素材索引（机读，详尽字段）
```

**manifest.json 字段定义**（详尽，便于引擎/脚本读取）：

```jsonc
{
  "project": "我的平台游戏",
  "exported_at": 1732200000,
  "format": "folder|zip",
  "total_files": 128,
  "total_bytes": 13002342,
  "files": [
    {
      "export_path": "assets/01_平台跳跃/Platformer_Art_Complete_Pack/sample.png",
      "category": "01_平台跳跃",
      "package": "Platformer_Art_Complete_Pack",
      "source_path": "D:\\Xiaoke\\GameAssets\\01_平台跳跃\\Platformer_Art_Complete_Pack\\sample.png",
      "name": "sample.png",
      "ext": "png",
      "kind": "image",
      "bytes": 12345,
      "width": 100,
      "height": 100,
      "frame_count": null,
      "content_hash": "sha256:..."
    }
  ]
}
```

字段覆盖：导出相对路径、分类、来源包、原始绝对路径、文件名、扩展名、kind 分类、字节数、尺寸（图片）、帧数（动画）、内容哈希。缺失字段用 null（如非图片的 width/height）。

**zip 模式**：用 Rust `zip` crate，文件流式写入。支持压缩级别（store 不压缩 / 默认 / 最高）。导出结构与文件夹模式一致（assets/ + CREDITS.* + manifest.json 全部打进 zip）。

**进度上报**：长任务通过 Tauri 事件 `export://progress` 推 `{job_id, stage, done, total, current_file}`，`stage` 取值 `copy|zip|credits|manifest`。前端显示进度条。

**幂等性**：重名导出目录/zip 提示覆盖/跳过/重命名，不静默覆盖。

---

## 5. 前端 UI 设计（Vue 3）

### 5.1 整体布局（三栏 + 任务面板）

```
┌──────────────────────────────────────────────────────────────┐
│ 顶栏: [库切换 ▼]  搜索框[____]  [扫描] [去重] [导出]          │
├────────────┬───────────────────────────┬─────────────────────┤
│ 左栏       │     中栏(主区域)           │   右栏              │
│ 分类树     │                            │                     │
│            │                            │   ┌───────────────┐ │
│ ▸01_平台   │   缩略图墙 / 文件列表       │   │ 选中清单       │ │
│ ▸02_地牢   │   ┌──┐┌──┐┌──┐┌──┐        │   │ (导出购物车)   │ │
▸ 04_图标UI  │   │  ││  ││  ││  │        │   │               │ │
│ ▾07_角色   │   └──┘└──┘└──┘└──┘        │   │ 3包 / 128文件  │ │
│   •ultim..│   ┌──┐┌──┐┌──┐┌──┐        │   │ ───────────── │ │
│   •LPC_.. │   │  ││  ││  ││  │        │   │ 估算 12.4 MB  │ │
│   •Dawn.. │   └──┘└──┘└──┘└──┘        │   │ [管理导出 ▶]  │ │
│ ▸13_3D模型 │                            │   └───────────────┘ │
│            │   [勾选模式: ○整包 ●文件]  │   ┌───────────────┐ │
│            │                            │   │ 预览面板        │ │
│            │                            │   │ (选中文件大图)  │ │
└────────────┴───────────────────────────┴─────────────────────┘
```

**三栏职责**：

- **左栏**：分类树（13 个一级目录），点击切换中栏。显示每类文件数/体积
- **中栏**：内容主区。包列表 → 点进包显示缩略图墙。支持网格/列表两种视图
- **右栏**：上方"选中清单"（导出购物车，实时统计），下方"预览面板"

### 5.2 核心交互流（混合粒度勾选）

**勾选模式切换**（中栏顶部）：

- `整包模式`：缩略图墙每个包左上角一个勾选框，勾上 = 整包导出
- `文件模式`：点进包内，每个文件卡片有独立勾选框

**勾选状态机**（一个包内文件被部分勾选时）：

```
包未勾选，0 文件勾  → ▢ 空框
包整勾，或全部文件勾 → ☑ 实勾
包勾了，部分文件勾   → ☑ 但子项有独立勾  → 包算"含部分文件勾选"
显式排除某文件        → 该文件在导出时跳过
```

**选中清单实时联动**：勾选变化 → 右栏立即重算"X 包 / Y 文件 / Z MB"。点"管理导出"打开导出面板。

### 5.3 预览组件（按通用预览器 viewer 路由）

前端按文件的 `viewer` 字段路由到固定组件，与 kind 解耦——新增资源类型只需在注册表映射到现有 viewer。

| viewer | 组件 | 行为 |
|---|---|---|
| `image` | `<ThumbImage>` | 棋盘格透明背景，点击放大 |
| `animated` | `<GifPlayer>` | 缩略图静止，点开逐帧播放，可调速度 |
| `vector` | `<SvgView>` | 直接内联 SVG |
| `audio` | `<AudioPlayer>` | 波形图 + 播放控件 |
| `font` | `<FontPreview>` | 渲染样例字形表 |
| `text` | `<TextPreview>` | 语法高亮只读，大文件只读首部 |
| `3d` | `<ModelViewer>` | Three.js 视口，旋转/缩放/线框模式 |
| `binary-source` | `<SourcePlaceholder>` | 文件名+大小+"需 XX 打开" |
| `fallback` | `<FileIcon>` | 图标 + 文件名/大小 |

组件数量固定（9 个），新增类型无需新增组件。

**性能要点**：缩略图墙用**虚拟滚动**（@tanstack/vue-virtual），4.2 万文件不能全渲染 DOM。

### 5.4 导出面板（模态/抽屉）

```
┌─ 导出项目 ────────────────────────────────┐
│ 项目名: [我的平台游戏____]                 │
│ 导出到: [D:\Projects\MyGame\art] [浏览]    │
│                                            │
│ 导出格式: ○ 文件夹   ● zip 压缩包           │
│ 压缩级别: ○存储(快) ●默认 ○最高(小)        │
│                                            │
│ 包含内容: 3 包 / 128 文件 / 12.4 MB        │
│ ☑ 复制为干净文件夹结构                      │
│ ☑ 生成 CREDITS.txt (版权署名)              │
│ ☑ 生成 manifest.json (素材索引)            │
│                                            │
│ ⚠ 2 个包缺版权信息 [去补录]                 │
│                                            │
│        [取消]  [开始导出 ▶]                │
└────────────────────────────────────────────┘
```

导出时显示进度条（接收 `export://progress` 事件，含 stage 阶段），完成后可"打开导出目录"。

### 5.5 设置页（类型注册表管理）

支撑"资源类型可扩充"的关键 UI。入口在顶栏设置按钮。

```
┌─ 设置 / 资源类型 ─────────────────────────────────┐
│ 内置类型(13)  自定义类型(2)            [+ 新增类型] │
│ ┌──────────────────────────────────────────────┐ │
│ │ kind        显示名   扩展名          预览器    │ │
│ │ image       图片     png,jpg,...     image     │ │
│ │ audio       音频     ogg,mp3,wav     audio     │ │
│ │ model3d     3D模型   obj,fbx,...     3d        │ │
│ │ ...                                          │ │
│ └──────────────────────────────────────────────┘ │
│ ⚠ 扫描发现 3 个未识别扩展名: webm, m4a, terr      │
│   [webm] → [新增为 video 类型]  [忽略]            │
│                                                   │
│ 编辑类型时改扩展名/预览器后，需 [重新分类全库]      │
└───────────────────────────────────────────────────┘
```

功能：列出内置+自定义类型；新增/编辑/删除自定义类型；把扫描发现的未知扩展名一键转为新类型；修改后触发重新分类。

### 5.6 前端状态管理

- **store/libraryStore**：库/分类/包数据，调用 indexer command
- **store/selectionStore**：勾选状态，与后端 selections 表同步
- **store/exportStore**：项目/导出任务/进度
- **store/typesStore**：类型注册表（内置默认+覆盖合并），预览器路由依据

---

## 6. 技术风险与实施

### 6.1 已识别的技术风险

| 风险 | 影响 | 应对 |
|---|---|---|
| **3D 格式转换**（fbx/dae→glb） | Rust 侧无成熟的 fbx/dae 转 glb 库 | MVP 用 assimp 绑定；若不理想，回退为"obj/gltf/glb 可预览，fbx/dae 仅显示信息"，blend 走本机 Blender 转 |
| **首次全扫 4.2 万文件耗时** | 用户首次等待长 | 并行扫描 + 实时进度事件；预计数十秒~1分钟内 |
| **GIF 拆帧 + 3D 转换占空间** | 缓存膨胀 | 缓存设上限（如 2GB），LRU 清理；懒生成（点开才转） |
| **`_来源.txt` 覆盖率仅 22/115** | 版权清单不完整 | 导出前提示缺失项 + 提供批量补录 UI |
| **中文路径编码**（Git Bash 乱码） | 文件操作路径错误 | Rust 用 OS 原生 PathBuf（UTF-16），不经 shell；前端用 convertFileSrc |
| **本机 Blender 缺失** | .blend 无法预览 | 检测 Blender 是否安装，缺失时优雅降级为"仅显示文件信息"，引导用户安装 |
| **音频解码**（ogg/wav 波形） | Rust 音频解码库成熟度参差 | wav 用 hound；ogg 用 vorbis 绑定；解码失败则 fallback 为"仅文件信息"，播放交前端 `<audio>` |
| **资源类型持续扩充** | 新格式需改代码才能支持 | 已通过可配置类型注册表（3.2）+ 通用预览器（3.3）解决：新增类型改配置不改代码 |

### 6.2 实施里程碑

虽然目标是完整版，实施仍按依赖顺序分里程碑推进，每个里程碑都是可运行状态：

```
M1 索引底座    类型注册表+扫描+SQLite索引+库管理  ← 让数据能进来
M2 浏览基础    分类/包/文件三栏 + 静态缩略图        ← 能看库了(最小可用)
M3 勾选+导出   混合勾选+复制+zip+credits+manifest  ← 核心闭环跑通
─────────────  ↑ 以上是核心闭环 ─────────────
M4 多类型预览  GIF帧/SVG/音频/字体/文本/预览器增强   ← 体验提升
M5 去重        三种检测+软删除trash                ← 整理能力
M6 3D预览      Three.js视口+格式转换+blend         ← 高阶能力
M7 类型管理    设置页类型注册表UI+未知类型提示+重新分类 ← 扩展性闭环
```

每个里程碑结束都能跑、能演示，避免"做完一大块发现方向偏了"。

> 注：M1 已内置完整默认类型表（开箱即用），M7 是让用户自助管理类型的 UI 闭环。即使不做 M7，软件也能正确识别现有全部格式并支持混合勾选导出。

### 6.3 不做（YAGNI 明确排除）

- ❌ 在线下载新素材（库已存在，不做采集）
- ❌ 素材编辑/PSD 分层编辑（那是 Photoshop 的活）
- ❌ 多用户/云同步（个人桌面工具）
- ❌ AI 自动打标签/识别内容（超出范围，且不可靠）

---

## 7. 验收标准（按里程碑）

| 里程碑 | 验收点 |
|---|---|
| M1 | 内置默认类型表覆盖库内全部格式（ogg/ttf/tmx/ase 等都被正确识别，非 other）；添加 GameAssets 库后，全量扫描能在 1 分钟内完成，ScanReport 文件数接近 4.2 万；二次启动秒开 |
| M2 | 三栏布局正常，点分类看包，点包看缩略图墙；图片缩略图正确显示棋盘格透明背景 |
| M3 | 混合勾选整包/文件可用；导出到文件夹和 zip 均成功；CREDITS.txt 含已知协议素材；manifest.json 字段完整 |
| M4 | GIF 能逐帧播放；SVG 正确渲染；音频可播放+波形图；字体显示字形；文本可查看内容 |
| M5 | 检测出已知的 zip+解压并存；备份目录被识别；软删除后文件移入 trash 可恢复 |
| M6 | obj/gltf/glb 在 Three.js 视口可旋转查看；blend 经本机 Blender 转换后可预览（装了 Blender 时） |
| M7 | 设置页可新增/编辑/删除自定义类型；扫描发现的未知扩展名能一键转为新类型；修改注册表后重新分类生效 |
