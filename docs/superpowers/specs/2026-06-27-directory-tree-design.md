# 目录树视图（Directory Tree View）设计

> 日期：2026-06-27
> 目标：无论用户如何组织资源目录，选择库后都能以完整目录树展示，适配任意深度嵌套结构。

## 1. 背景与问题

当前应用假设资源库是 `分类/包/文件` 三级固定结构：
- 一级目录 = 分类（categories 表）
- 二级目录 = 包（packages 表）
- 更深层 = 文件

这套结构对精心整理过的库（如用 AI 规整的 `01_2D像素/素材包A/...`）很整齐。但普通用户下载的资源往往是任意深度嵌套，硬塞进两级会丢失真实结构，难以浏览。

## 2. 设计目标

- **完整目录树**：以真实目录结构展示，任意深度，叶子/中间节点都可见。
- **每个含文件的目录都是"包"**：点开能看到它直接含的文件（类似文件管理器）。
- **只读展示**：树只用于浏览，不做移动/重命名/拖拽排序。
- **如实反映磁盘**：用户在磁盘上增/删/改文件夹后，应用自动同步（3-5 秒内，经现有 watcher）。
- **与现有两级视图并存**：不破坏 categories/packages 模型，旧视图零风险保留。

## 3. 数据模型

### 3.1 新增 directories 表

```sql
CREATE TABLE directories (
  id          INTEGER PRIMARY KEY,
  library_id  INTEGER NOT NULL REFERENCES libraries(id) ON DELETE CASCADE,
  parent_id   INTEGER REFERENCES directories(id) ON DELETE CASCADE,  -- NULL=根级目录
  name        TEXT NOT NULL,          -- 目录名（如 "树木"）
  path        TEXT NOT NULL,          -- 相对库根的完整路径（如 "05_自然环境/树木"）
  depth       INTEGER NOT NULL,       -- 0=库根下第一层
  file_count  INTEGER DEFAULT 0,      -- 该目录【直接】含的文件数（不含子目录递归）
  total_bytes INTEGER DEFAULT 0,      -- 直接含文件的总大小
  UNIQUE(library_id, path)
);
CREATE INDEX idx_dirs_parent ON directories(parent_id);
CREATE INDEX idx_dirs_library ON directories(library_id);
```

- **包 = 任何 file_count > 0 的目录**。点开它显示直接含的文件。
- parent_id 自引用实现树结构；ON DELETE CASCADE 保证删父目录时子目录一起删。

### 3.2 files 表扩展

```sql
ALTER TABLE files ADD COLUMN directory_id INTEGER REFERENCES directories(id) ON DELETE CASCADE;
```

- 文件挂到它**直接所在目录**的 directory_id。
- 保留现有 `package_id`（兼容旧两级视图和选择/导出逻辑），旧逻辑零影响。

### 3.3 现有表不动

`categories` / `packages` / `selections` / `duplicate_*` 全部不动。两级视图继续用它们。

## 4. 扫描入库

### 4.1 全量扫描（新增 directory 版）

新扫描逻辑 `scan_library_tree(root)` 替代"假设三级"的写法，用 walkdir 完整遍历：

1. 遍历整棵目录树，为每个目录建 directory 记录（parent_id / path / depth）。
2. 文件挂到直接所在目录的 directory_id。
3. 单事务批量写入（保持 4.2 万文件 ~5 秒性能）。
4. 跳过规则不变：`_下载脚本`、`.` 开头目录。

### 4.2 增量扫描（如实反映磁盘变化）

增量扫描对比 directories 表与磁盘实际目录树：

- **新增目录**：磁盘有、表里无 → 插入 directory 记录。
- **删除目录**：表里有、磁盘无 → **直接删除记录**（CASCADE 删其 files）。选择记录中关联的文件也会随之失效（符合"如实反映"，文件已不存在）。
- **重命名/移动**：旧 path 消失 + 新 path 出现 → 删旧建新（等价于删除+新增）。

关键：增量扫描同时维护 directories 和 files，不只管 files。

### 4.3 watcher 自动触发

现有 notify watcher（3 秒防抖）监听库根目录，检测到 Create/Modify/Remove（含目录变化）即触发增量扫描。用户磁盘改文件夹后 3-5 秒内应用自动反映，无需手动刷新。

## 5. 后端命令（新增 3 个，Tauri command）

```rust
// 取整棵目录树（嵌套 children 结构，含 file_count/total_bytes/depth）
get_directory_tree(lib_id) -> Vec<DirNode>

// 取某目录【直接】含的文件
get_directory_files(directory_id) -> Vec<FileNode>

// 取某目录及所有子目录的文件汇总（点根看全部）
get_subtree_files(directory_id) -> Vec<FileNode>
```

现有命令（get_categories / get_packages / get_package_files / search_files）全部保留。

## 6. 前端

### 6.1 左侧面板顶部切换

```
[📁 两级视图] [🌳 树视图]   ← 切换按钮，记忆选择（localStorage "vault.leftView"）
```

### 6.2 树视图组件（DirectoryTree.vue，新增）

- 递归渲染目录树，每层缩进。
- 展开/折叠（▶/▼），默认只展开第一层。
- 显示：目录名 + 文件数/大小（如 `树木 · 12 文件 · 3.2MB`）。
- 含文件的目录可点击 → 中间网格显示其直接文件。
- file_count=0 的纯分组目录也能展开看子项，但点它不显示文件。

### 6.3 中间网格复用

FileGrid 组件复用，数据源切换：
- 两级视图：files 来自 get_package_files(pkg_id)
- 树视图：files 来自 get_directory_files(dir_id)

虚拟滚动、缩略图、勾选、定位逻辑全部复用。

## 7. 勾选与导出（兼容）

- 树视图下勾选目录 = 勾选它直接含的所有文件（selections scope='file'）。
- 导出逻辑完全不变（selections 表结构不变）。
- 选择汇总栏（SelectionBar）逻辑不变，统计仍正确。

## 8. 边界与不做的事

- ❌ 不改 packages 表结构（旧两级视图零风险）
- ❌ 不做拖拽排序 / 重命名 / 移动文件（树只读）
- ❌ 不做目录的多选勾选（点目录=显示文件，勾选在网格里做）
- ✅ 搜索、去重、预览全部与新树视图共存

## 9. 实现风险

1. **增量扫描性能**：每次都要对比整棵目录树。4.2 万文件已验证 ~5 秒，目录树对比开销应可接受（目录数远少于文件数）。
2. **重命名误判**：重命名=删旧建新，若旧目录有选择记录会丢失。属"如实反映"的预期行为，可接受。
3. **数据迁移**：首次启动需建 directories 表 + 扫描入库。用 migrate() 自动完成，不影响现有数据。
