# XiaokeVault · 笑客宝库 — 开发进度

> 最后更新：2026-06-27
> 仓库：https://github.com/1504460699/XiaokeVault

## 当前状态：1.0 功能完整，待最终验证

- ✅ Backlog 12/12 全部完成
- ✅ 最终安装包已打包：`dist-release/XiaokeVault_0.1.0_x64-setup.exe`（4.1MB）
- ✅ 代码已全部提交并推送到 GitHub（HEAD: `a295138`）
- ⏳ **待办：用户安装最终版验证**（GIF 逐帧、音频波形、错误 Toast、目录树、i18n、X 图标）

---

## 已完成功能清单

### 核心功能（M1-M7 + backlog 全部）
| 功能 | 状态 |
|---|---|
| SQLite 索引（4.2 万文件 ~5 秒）+ 自动增量扫描（notify watcher） | ✅ |
| 🌳 目录树视图（任意深度，点文件夹显示子树文件，如实反映磁盘变化） | ✅ |
| 多格式预览：图片缩放/SVG/**GIF 逐帧**/**音频波形**/字体/文本/3D 模型 | ✅ |
| 混合粒度勾选 + 按项目打包导出（文件夹/zip + credits + manifest） | ✅ |
| 去重（zip/解压 + 疑似备份 + 持久化忽略） | ✅ |
| 类型可扩充管理（内置 13 类 + DB 覆盖 + 自定义） | ✅ |
| 全局跨包搜索 | ✅ |

### 工程质量
| 项 | 状态 |
|---|---|
| 🌐 中英文国际化（vue-i18n，顶栏切换，记忆选择） | ✅ |
| 结构化错误处理（后端 AppError thiserror + 前端统一 Toast 中文提示） | ✅ |
| 单元测试（17 个，dedup/exporter/asset_types，cargo test 全绿） | ✅ |
| 品牌：X 图标、笑客宝库/XiaokeVault 双语名、窗口标题跟随语言 | ✅ |
| UI：窗口按钮固定右上角、TopBar 不变形、长名悬停提示、右键禁用 | ✅ |
| MIT 开源许可证 + THIRD-PARTY-NOTICES.md | ✅ |
| app data 自动迁移（com.xiaoke.tauri-app → com.xiaoke.vault，保护现有数据） | ✅ |

---

## 技术栈
- **后端**：Rust + Tauri 2（sqlx/walkdir+rayon/notify/image/zip/thiserror）
- **前端**：Vue 3 + Pinia + Tailwind v4 + @tanstack/vue-virtual + Three.js + vue-i18n + gifuct-js

## 关键文件
- 品牌图标源：`brand/icon.svg`（X 字母）+ `brand/svg2png.mjs`（栅格化脚本）
- 重新生成图标：`pnpm tauri icon brand/icon-1024.png`
- 错误类型：`src-tauri/src/error.rs`（AppError 枚举）
- 国际化：`src/i18n/{zh,en}.ts`
- 目录树：`src-tauri/src/{tree_scanner,tree}.rs` + `src/components/{DirectoryTree,DirTreeNode}.vue`

---

## 明天可继续的方向（backlog 已清，以下为可选）

1. **用户验证最终安装包后**，根据反馈修 bug / 调细节
2. **正式发布 v1.0**：打 git tag、写 Release Notes、GitHub Releases 上传安装包
3. **新增 backlog 方向**（根据实际使用痛点）：
   - 性能优化（超大库的目录树加载）
   - 更多预览格式（视频 webm/mp4）
   - 标签/收藏功能
   - 主题切换（深/浅色）

## 验证最终安装包时重点检查
1. GIF 逐帧播放（进度条/调速/逐帧步进）
2. 音频波形图（点击跳转 + 已播放高亮）
3. 错误提示（顶部 Toast 中文，非英文 alert）
4. 目录树（点任意文件夹显示子树文件）
5. 语言切换（中文=笑客宝库，英文=XiaokeVault，标题跟随）
6. X 图标（任务栏/开始菜单）
