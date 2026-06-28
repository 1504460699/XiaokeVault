# 第三方开源许可声明 (Third-Party Notices)

XiaokeVault · 笑客宝库 使用了以下开源库，在此致谢。各库的完整许可证文本请见其官方仓库。

本应用基于 **MIT License** 开源。下方列出的第三方库同样使用宽松许可证（MIT / Apache-2.0 / BSD / ISC），其版权声明在此保留。

---

## 前端（Frontend）

| 库 | 许可证 | 用途 |
|---|---|---|
| [Vue.js](https://vuejs.org/) | MIT | 前端框架 |
| [Pinia](https://pinia.vuejs.org/) | MIT | 状态管理 |
| [Vue I18n](https://vue-i18n.intlify.dev/) | MIT | 国际化 |
| [Tailwind CSS](https://tailwindcss.com/) | MIT | CSS 框架 |
| [Vite](https://vitejs.dev/) | MIT | 构建工具 |
| [@tanstack/vue-virtual](https://tanstack.com/virtual) | MIT | 虚拟滚动 |
| [Three.js](https://threejs.org/) | MIT | 3D 模型预览 |
| [fflate](https://github.com/101arrowz/fflate) | MIT | FBX 解压 |

## 后端（Backend / Rust）

| 库 | 许可证 | 用途 |
|---|---|---|
| [Tauri](https://tauri.app/) | Apache-2.0 / MIT | 桌面应用框架 |
| [Serde](https://serde.rs/) | MIT / Apache-2.0 | 序列化 |
| [SQLx](https://github.com/launchbadge/sqlx) | MIT / Apache-2.0 | SQLite 数据库 |
| [Tokio](https://tokio.rs/) | MIT | 异步运行时 |
| [walkdir](https://github.com/BurntSushi/walkdir) | MIT / Unlicense | 目录遍历 |
| [Rayon](https://github.com/rayon-rs/rayon) | MIT / Apache-2.0 | 并行计算 |
| [Chrono](https://github.com/chronotope/chrono) | MIT / Apache-2.0 | 时间处理 |
| [image](https://github.com/image-rs/image) | MIT / Apache-2.0 | 缩略图生成 |
| [zip](https://github.com/zip-rs/zip) | MIT | zip 压缩导出 |
| [notify](https://github.com/notify-rs/notify) | CC0-1.0 | 文件监听（自动扫描）|
| [log](https://github.com/rust-lang/log) | MIT / Apache-2.0 | 日志 |

## Tauri 官方插件

| 库 | 许可证 | 用途 |
|---|---|---|
| tauri-plugin-dialog | MIT / Apache-2.0 | 原生对话框 |
| tauri-plugin-opener | MIT / Apache-2.0 | 打开文件夹 |
| tauri-plugin-log | MIT / Apache-2.0 | 日志输出 |

---

> 完整的传递依赖（含每个库的子依赖）可通过 `pnpm licenses list`（前端）和 `cargo tree`（后端）查看，它们的许可证声明随源码分发自动包含。

---

## 许可证摘要

- **MIT License**：可自由使用、修改、分发、商用，需保留版权声明。
- **Apache-2.0**：同上，另含专利授权条款，需保留 NOTICE 文件。
- **BSD-2/3-Clause**：可自由使用，需保留版权声明，不得用作者名背书。
- **ISC License**：等同于 MIT 的宽松许可。
- **CC0-1.0**：公共领域，无任何限制。

所有上述许可证都允许在 MIT 项目中使用和再分发。
