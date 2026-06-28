# XiaokeVault 后续开发升级方案 · ROADMAP

> 版本：基于 v1.0.0（2026-06-28 发布）
> 目的：记录后续可迭代的功能方向、技术方案、优先级与工作量评估
> 维护：按实际使用反馈滚动更新

---

## 一、优先级总览

| 优先级 | 方向 | 工作量 | 价值 |
|---|---|---|---|
| **P0** | 版权元数据编辑 UI | 小 | 让 CREDITS 导出真正可用 |
| **P0** | 隐藏目录扫描修复 | 小 | 修复 .mayaSwatches 等跳过问题 |
| **P1** | 视频预览（mp4/webm） | 中 | 补齐多媒体预览短板 |
| **P1** | 按目录重新实现去重 | 大 | 恢复被移除的实用功能 |
| **P1** | 标签 / 收藏夹 | 中 | 跨目录组织资产 |
| **P2** | macOS / Linux 适配 | 中 | 扩大平台覆盖 |
| **P2** | 超大库性能优化 | 中 | 支撑 10 万+ 文件 |
| **P2** | 主题切换（深/浅色） | 小 | 视觉舒适度 |
| **P3** | 拖拽导出 | 小 | 操作便捷 |
| **P3** | 批量重命名 | 中 | 资产整理 |
| **P3** | 相似图片检测 | 大 | 视觉去重 |
| **P3** | Git 集成 | 中 | 资产版本追踪 |

---

## 二、P0 — 体验补全（建议立即做）

### 2.1 版权元数据编辑 UI

**背景**：0006 迁移已给 `directories` 表加了 `source_url / source_title / license / license_source` 四列，exporter 的 `build_credits()` 已改为从 directories 查询。但**前端没有编辑入口**，导致导出时 CREDITS 全为空。

**方案**：
1. 新增后端命令 `update_directory_meta(dir_id, source_url, source_title, license, license_source)`
2. 新增 `src/components/DirMetaDialog.vue`：在 DirectoryTree 右键目录 →「编辑属性」打开弹窗
3. 弹窗含 4 个输入框 + 保存按钮，保存后 UPDATE directories
4. exporter 的 CREDITS 即可读到真实数据

**改动文件**：
- `src-tauri/src/tree.rs`：加 `update_directory_meta` 命令
- `src-tauri/src/lib.rs`：注册命令
- `src/ipc/library.ts`：加 `updateDirectoryMeta` 封装
- `src/components/DirMetaDialog.vue`（新）
- `src/components/DirectoryTree.vue`：右键菜单入口
- i18n 文案

**工作量**：约半天

---

### 2.2 隐藏目录扫描修复

**背景**：当前 `.mayaSwatches` 等「点开头」的隐藏目录会被 tree_scanner 扫描到文件，但其目录本身未被加入 `path_to_id`，导致这些文件因 `dir_id=None` 被跳过（日志可见 3 个文件被跳过）。

**根因**：`tree_scanner.rs` 收集目录时跳过了 `.开头` 目录，但收集文件时没跳过 → 数据不一致。

**方案**：统一在 `tree_scanner.rs` 的 `should_skip` 判断里处理——
- 选项 A：扫描文件时也跳过隐藏目录内的文件（隐藏目录本就是缓存，不入库合理）
- 选项 B：扫描目录时也收集隐藏目录（完整入库）

**推荐 A**：游戏素材库的 `.mayaSwatches`、`.DS_Store` 等本就是垃圾缓存，跳过更干净。

**改动文件**：
- `src-tauri/src/tree_scanner.rs`：`scan_subtree` 里文件收集加 `should_skip` 判断

**工作量**：1 小时

---

## 三、P1 — 核心功能扩展

### 3.1 视频预览（mp4/webm）

**背景**：当前支持图片/GIF/音频/文本/3D/字体，缺视频。游戏资产常有动画演示视频。

**方案**：
1. asset_types 内置 `video` 类型，扩展名 `mp4/webm/mov/avi`
2. 新增 `src/components/preview/VideoPlayer.vue`：用 HTML5 `<video>` 标签
   - 播放/暂停、进度条、音量、全屏
   - 缩略图：用 ffmpeg 抽首帧（或 `<video>` + canvas 截图）
3. viewer router：`video` → VideoPlayer

**改动文件**：
- `src-tauri/src/asset_types.rs`：`builtin_types()` 加 video
- `src/components/preview/VideoPlayer.vue`（新）
- `src/components/PreviewPane.vue`：viewer 映射加 video
- `src/utils/viewer.ts`：canShowThumb 加 video

**工作量**：1-2 天（含抽帧，若用 ffmpeg 需额外处理依赖）

**技术注意**：CSP 的 `media-src` 已含 `asset:`，无需改 CSP。

---

### 3.2 按目录重新实现去重

**背景**：v1.0.0 因两级视图移除而删除了 dedup 功能。但「zip + 解压目录」「相似备份」的检测对资产整理很有价值，值得按目录树重新实现。

**方案**（基于目录树，不再依赖 packages）：
1. **检测维度改为 directory**：
   - 同一父目录下，存在「xxx.zip」和同名「xxx/」目录 → 疑似压缩包+解压
   - 同一父目录下，两个子目录名高度相似（如 `hero_v1`、`hero_v2`）→ 疑似版本备份
2. **新建表**（替代旧 dedup 表，不引用 packages）：
   ```sql
   CREATE TABLE duplicate_groups (id, library_id, reason, created_at);
   CREATE TABLE duplicate_members (id, group_id, directory_id, role, UNIQUE(group_id, directory_id));
   ```
3. 后端 `dedup.rs` 重写：SQL 改为查 directories 表的父子关系
4. 前端 `DedupPanel.vue` 重写：定位逻辑改为 `tree.selectDirectory()`
5. 持久化忽略：`dismissed_pairs` 改为存 `directory_a/directory_b`

**改动文件**：
- `src-tauri/src/dedup.rs`（重写）
- `src-tauri/migrations/0007_dedup_v2.sql`（新，不依赖 packages）
- `src/components/DedupPanel.vue`（重写）
- `src/stores/dedupStore.ts`（重写）
- `src/types/dedup.ts`（重写）
- lib.rs / ipc / i18n 恢复相关注册

**工作量**：3-5 天（重写量较大，但逻辑可复用旧版思路）

---

### 3.3 标签 / 收藏夹

**背景**：目录树按物理位置组织，但用户常需要「按主题/用途」跨目录组织资产（如「所有 UI 图标」「所有 boss 角色」）。

**方案**：
1. 新建表：
   ```sql
   CREATE TABLE tags (id, name, color, created_at);
   CREATE TABLE file_tags (file_id, tag_id, UNIQUE(file_id, tag_id));
   ```
2. 后端命令：`create_tag / list_tags / toggle_file_tag / get_files_by_tag`
3. 前端：
   - 左侧新增「标签」侧边栏（与目录树并列切换）
   - FileGrid 文件卡片显示标签小圆点
   - 右键文件 → 添加/移除标签
   - 收藏夹 = 特殊的「⭐ 收藏」标签

**改动文件**：
- `src-tauri/migrations/0008_tags.sql`（新）
- `src-tauri/src/tags.rs`（新）
- `src/components/TagSidebar.vue`（新）
- 各组件加标签显示

**工作量**：3-4 天

---

## 四、P2 — 性能与扩展

### 4.1 macOS / Linux 适配

**现状**：Tauri 本身跨平台，主要差异在：
- 路径分隔符（`\` vs `/`）—— 已在多处用 `.replace('\\','/')` 处理，需全局审计
- 文件监听（notify 库跨平台，但 macOS 用 FSEvents，行为略有差异）
- appdata 路径（`dirs` crate 已处理）
- 图标格式（macOS 用 .icns）

**方案**：
1. 全局审计所有硬编码 `\` 或 Windows 路径假设
2. CI 加 macOS/Linux 构建矩阵
3. 测试 notify watcher 在 macOS 的行为

**工作量**：2-3 天（主要是测试 + 修兼容问题）

---

### 4.2 超大库性能优化

**现状**：4.2 万文件 + 1500 目录，扫描 5.8 秒，目录树一次性加载所有节点。10 万+ 文件时目录树渲染会卡。

**方案**：
1. **目录树懒加载**：`get_directory_tree` 改为只返回顶层 + 展开时按需加载子节点
2. **目录树虚拟滚动**：用 `@tanstack/vue-virtual` 虚拟化树节点（当前只虚拟化了 FileGrid）
3. **文件网格分页**：超 5000 文件的目录改为分页/虚拟列表（当前已是虚拟列表，但 filteredFiles 计算可能慢）
4. **搜索索引**：文件名搜索改用 SQLite FTS5 全文索引（替代 LIKE），4 万级数据提升明显

**工作量**：按需，每个点 1-2 天

---

### 4.3 主题切换（深/浅色）

**现状**：当前固定深色（`bg-slate-900`）。

**方案**：Tailwind v4 dark mode + CSS 变量
1. 定义 `:root` 和 `[data-theme="light"]` 两套颜色变量
2. 把所有 `bg-slate-900` 等改为 `bg-bg-primary`（语义类）
3. 顶栏加主题切换按钮，记忆到 localStorage

**工作量**：1-2 天（主要是替换所有颜色类）

---

## 五、P3 — 锸上添花

### 5.1 拖拽导出
- FileGrid 文件卡片支持拖拽，拖到资源管理器直接复制
- Tauri 2 的 `tauri-plugin-drag` 或自实现 `onDragStart`

### 5.2 批量重命名
- 选中多个文件 → 规则替换（正则/序号/日期）
- 后端 `batch_rename` 命令，带撤销

### 5.3 相似图片检测
- 基于感知哈希（pHash）：扫描时计算每图 pHash，存 `files.content_hash` 列（已存在但未用）
- Hamming 距离 < 阈值判定相似
- 复用去重 UI 框架

### 5.4 Git 集成
- 检测库根是否为 git 仓库（`.git` 存在）
- 用 `git2` crate 显示文件 modified/untracked 状态
- FileGrid 文件角标显示 git 状态

---

## 六、技术债务清单

| 债务 | 严重度 | 说明 |
|---|---|---|
| thumbs 目录有旧 id 残留（80000+） | 低 | 迁移后旧缩略图未清理，占空间但不影响功能 |
| asset_types 表为空（全靠内置） | 低 | 用户自定义类型会写入，但当前 0 行说明首次未种子化 |
| 0001/0002/0003 迁移文件为「补丁式」 | 中 | 经历多次重写，建议未来用「全新库初始化脚本」替换，历史迁移冻结 |
| log:: 宏与 alog_! 宏混用 | 低 | 旧代码用 log::，新代码用 alog_!，可统一 |
| 无 gh CLI | 低 | Release 手动操作，建议装 gh 实现一键发版 |

---

## 七、发版流程建议

### 当前流程（手动）
1. 改版本号（package/tauri/cargo）
2. `npm run tauri build`
3. 复制安装包到 dist-release
4. git commit + tag + push
5. 浏览器手动创建 GitHub Release

### 建议优化（一键发版）
1. 安装 `gh` CLI
2. 写 `scripts/release.sh`：
   ```bash
   VERSION=$1
   # 改版本号
   npm run tauri build
   cp src-tauri/target/release/bundle/nsis/XiaokeVault_${VERSION}_x64-setup.exe dist-release/
   git add -A && git commit -m "release: v${VERSION}" && git tag v${VERSION} && git push
   gh release create v${VERSION} dist-release/XiaokeVault_${VERSION}_x64-setup.exe \
     --notes-file dist-release/RELEASE_NOTES.md --title "XiaokeVault v${VERSION}"
   ```
3. 以后只需 `./scripts/release.sh 1.1.0`

---

## 八、结语

v1.0.0 是一个稳定可用的首版，覆盖了「扫描入库 → 浏览预览 → 搜索定位 → 勾选导出」的完整工作流。
后续迭代建议**以实际使用痛点驱动**，优先做 P0（版权 UI、隐藏目录修复）补齐体验，
再按需做 P1 的视频预览/去重/标签。性能优化（P2）等库规模增长后再做。

保持「小步快跑、每个版本可独立交付价值」的节奏。
