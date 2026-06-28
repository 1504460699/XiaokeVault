# XiaokeVault · 笑客宝库

![License: MIT](https://img.shields.io/badge/License-MIT-skyblue.svg)
![Version: v1.0.0](https://img.shields.io/badge/Version-v1.0.0-skyblue.svg)
![Platform: Windows](https://img.shields.io/badge/Platform-Windows-slate.svg)
![Built with Tauri](https://img.shields.io/badge/Built%20with-Tauri-orange.svg)

> 游戏素材资产管理工具 —— 把散乱的素材目录，整理成可检索、可预览、可打包导出的宝库。

## 这是什么

下载的游戏素材往往散乱在成百上千个目录里。XiaokeVault 解决的就是这件事：

- **目录树管理**：按真实文件夹结构建立索引，如实反映磁盘层级，4.2 万文件约 5 秒完成。
- **多格式预览**：图片（可缩放）、SVG、GIF 逐帧、音频、字体、文本、3D 模型（glb/gltf/obj/dae/fbx/blend）。
- **全局实时搜索**：跨目录按文件名搜索，输入即搜（500ms 防抖），结果可定位到所在目录。
- **混合粒度勾选 + 导出**：按目录（含子目录）或按单个文件勾选，导出成干净目录结构 + `CREDITS` + `manifest.json`，支持文件夹或 zip。
- **类型可扩充**：内置 13 类素材类型，可在「类型」面板里增删改，无需改代码。
- **自动增量扫描**：监听资源目录变化，新增/修改/删除文件自动入库（防抖触发）。
- **双语界面**：中文 / English 一键切换。

## 下载安装

前往 [Releases](https://github.com/1504460699/XiaokeVault/releases) 下载 `XiaokeVault_1.0.0_x64-setup.exe`（约 4MB），双击安装即可。

- 系统要求：Windows 10/11 (x64)
- 无需预装运行时（WebView2 随系统自带）

## 技术栈

- **后端**：Rust + Tauri 2（sqlx / walkdir+rayon / notify / image / zip / thiserror）
- **前端**：Vue 3 + Pinia + Tailwind v4 + @tanstack/vue-virtual + Three.js + vue-i18n + gifuct-js

## 开发

环境要求：[Node.js](https://nodejs.org/)、[Rust](https://www.rust-lang.org/)、[pnpm](https://pnpm.io/)

```bash
pnpm install
pnpm tauri dev      # 开发模式
pnpm tauri build    # 打包安装程序
```

## 使用方式

1. 启动后点「+ 添加库」，选择游戏素材根目录
2. 点「扫描」建立索引（首次约 5 秒，4 万文件级别）
3. 左侧目录树浏览，点击文件在右侧预览
4. 勾选需要的文件或整个目录，点「导出」打包

## 资源库目录约定

- 库根目录下任意层级的真实文件夹结构都会被如实建为目录树
- `_` 下划线开头的目录会被跳过（用于存放下载脚本等非素材内容）
- 隐藏目录（`.` 开头，如 `.mayaSwatches`）会被跳过

## 许可证

本项目基于 [MIT License](LICENSE) 开源，可自由使用、修改和分发（含商用），请保留版权声明。

本项目使用了多个开源库，详见 [第三方许可声明](THIRD-PARTY-NOTICES.md)。
