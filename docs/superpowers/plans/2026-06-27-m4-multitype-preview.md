# M4 多类型预览 实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 把缩略图墙和预览面板的"📦占位"替换成真正的格式预览：GIF 动画、SVG 矢量、音频播放、字体字形、文本内容。让所有主要 2D 资源类型都能可视化。

**Architecture:** 全部前端实现，不需 Rust 后端改动。核心是按 file.viewer 路由到不同的预览组件，复用 M2 的 convertFileSrc 加载本地文件。viewer 字段从 kind 映射（前端维护映射表）。

**Tech Stack:** Vue 3 组件，原生 `<audio>`/`<img>`，fetch 读取文本

**Spec:** `docs/superpowers/specs/2026-06-27-game-asset-manager-design.md` §3.3 通用预览器, §5.3 预览组件

**前置条件:** M1（kind 分类）、M2（缩略图墙 + convertFileSrc）、M3（预览面板）已完成

---

## 关键决策：viewer 映射放前端

设计 §3.2 的 kind 已存在 files 表。viewer 是 kind→预览器的映射（§3.3）。M4 在前端维护一张 `kind → viewer` 映射表（与设计 §3.2 内置表的 viewer 列一致），缩略图和预览都按 viewer 路由。

```ts
// kind → viewer（与设计 §3.2 内置表 viewer 列对齐）
const VIEWER_BY_KIND: Record<string, string> = {
  image: "image", animated: "animated", vector: "vector",
  audio: "audio", font: "font", text: "text",
  model3d: "3d",
  source_blend: "binary-source", source_pixel: "binary-source", source_design: "binary-source",
  archive: "fallback", legacy_media: "fallback", other: "fallback",
};
```

缩略图策略（缩略图墙里小尺寸预览）：
- image/vector/animated → 用 convertFileSrc 显示（GIF 显示首帧即可，浏览器默认显示首帧）
- 其他 → 图标占位（详细预览在右栏）

---

## 文件结构

```
src/
├── utils/
│   └── viewer.ts              # kind→viewer 映射 + 图标
├── components/
│   └── preview/
│       ├── TextPreview.vue    # 文本查看（xml/tmx/json/txt）
│       ├── AudioPlayer.vue    # 音频播放（ogg/mp3/wav）
│       ├── FontPreview.vue    # 字体字形（ttf/otf）
│       └── SourcePlaceholder.vue  # 源文件占位（xcf/psd/ase）
├── components/
│   ├── FileGrid.vue           # 改：缩略图按 viewer 路由
│   └── PreviewPane.vue        # 改：预览按 viewer 路由
└── ipc/
    └── fileUrl.ts             # 已有 convertFileSrc
```

---

## Task 1: viewer 映射工具

**Files:**
- Create: `src/utils/viewer.ts`

- [ ] **Step 1: 创建 src/utils/viewer.ts**

```ts
// kind → viewer 映射（与设计 §3.2 内置表 viewer 列对齐）
const VIEWER_BY_KIND: Record<string, string> = {
  image: "image",
  animated: "animated",
  vector: "vector",
  audio: "audio",
  font: "font",
  text: "text",
  model3d: "3d",
  source_blend: "binary-source",
  source_pixel: "binary-source",
  source_design: "binary-source",
  archive: "fallback",
  legacy_media: "fallback",
  other: "fallback",
};

export function viewerForKind(kind: string): string {
  return VIEWER_BY_KIND[kind] ?? "fallback";
}

// viewer → 占位图标（emoji，缩略图墙用）
const ICON_BY_VIEWER: Record<string, string> = {
  audio: "🎵",
  font: "🔤",
  text: "📄",
  "3d": "🧊",
  "binary-source": "⚙️",
  fallback: "📦",
};

export function iconForViewer(viewer: string): string {
  return ICON_BY_VIEWER[viewer] ?? "📦";
}

// 缩略图墙里能直接用 img 显示的 viewer
export function canShowThumb(viewer: string): boolean {
  return viewer === "image" || viewer === "animated" || viewer === "vector";
}
```

- [ ] **Step 2: 验证编译**

Run: `npm run build`
Expected: 通过。

- [ ] **Step 3: Commit**

```bash
git -C "D:/Xiaoke/Works/Tools/XiaokeTools" add -A
git -C "D:/Xiaoke/Works/Tools/XiaokeTools" commit -m "feat(m4): viewer mapping utility (kind->viewer, icons, thumb eligibility)"
```

---

## Task 2: FileGrid 缩略图按 viewer 路由

缩略图墙：image/animated/vector 显示文件图，其他显示图标。

**Files:**
- Modify: `src/components/FileGrid.vue`

- [ ] **Step 1: FileGrid 改用 viewerForKind**

在 `<script setup>` import：

```ts
import { viewerForKind, iconForViewer, canShowThumb } from "../utils/viewer";
```

模板里缩略图区域改：

```html
<div class="flex-1 flex items-center justify-center bg-slate-900 overflow-hidden">
  <img
    v-if="canShowThumb(viewerForKind(f.kind))"
    :src="getFileUrl(f)"
    class="max-w-full max-h-full object-contain"
    loading="lazy"
  />
  <div v-else class="text-3xl">{{ iconForViewer(viewerForKind(f.kind)) }}</div>
</div>
```

移除原来的 `isImage` 函数（被 viewerForKind 取代）。

- [ ] **Step 2: 验证编译**

Run: `npm run build`
Expected: 通过。

- [ ] **Step 3: Commit**

```bash
git -C "D:/Xiaoke/Works/Tools/XiaokeTools" add -A
git -C "D:/Xiaoke/Works/Tools/XiaokeTools" commit -m "feat(m4): FileGrid thumbnails routed by viewer"
```

---

## Task 3: TextPreview 组件（文本查看）

读文本文件内容显示。用 fetch + convertFileSrc（asset 协议支持 fetch）。

**Files:**
- Create: `src/components/preview/TextPreview.vue`

- [ ] **Step 1: 创建 src/components/preview/TextPreview.vue**

```vue
<script setup lang="ts">
import { ref, watch } from "vue";
import { getFileUrl } from "../../ipc/fileUrl";
import type { FileNode } from "../../types/library";

const props = defineProps<{ file: FileNode }>();
const content = ref("");
const loading = ref(false);
const error = ref<string | null>(null);
const TRUNCATE_BYTES = 50000;

watch(
  () => props.file,
  async (f) => {
    loading.value = true;
    error.value = null;
    content.value = "";
    try {
      const url = getFileUrl(f);
      const res = await fetch(url);
      if (!res.ok) throw new Error("HTTP " + res.status);
      const text = await res.text();
      content.value =
        text.length > TRUNCATE_BYTES
          ? text.slice(0, TRUNCATE_BYTES) + `\n\n…（已截断，共 ${text.length} 字符）`
          : text;
    } catch (e) {
      error.value = String(e);
    } finally {
      loading.value = false;
    }
  },
  { immediate: true },
);
</script>

<template>
  <div class="w-full text-xs text-slate-300 font-mono">
    <div v-if="loading" class="text-slate-500">读取中…</div>
    <div v-else-if="error" class="text-red-400">读取失败：{{ error }}</div>
    <pre v-else class="whitespace-pre-wrap break-all max-h-80 overflow-auto bg-slate-900 p-2 rounded">{{ content }}</pre>
  </div>
</template>
```

- [ ] **Step 2: Commit**

```bash
git -C "D:/Xiaoke/Works/Tools/XiaokeTools" add -A
git -C "D:/Xiaoke/Works/Tools/XiaokeTools" commit -m "feat(m4): TextPreview component"
```

---

## Task 4: AudioPlayer 组件（音频播放）

用原生 `<audio>` 标签 + convertFileSrc。

**Files:**
- Create: `src/components/preview/AudioPlayer.vue`

- [ ] **Step 1: 创建 src/components/preview/AudioPlayer.vue**

```vue
<script setup lang="ts">
import { getFileUrl } from "../../ipc/fileUrl";
import type { FileNode } from "../../types/library";

const props = defineProps<{ file: FileNode }>();
</script>

<template>
  <div class="w-full flex flex-col items-center gap-2 py-4">
    <div class="text-5xl">🎵</div>
    <div class="text-xs text-slate-400 truncate max-w-full">{{ props.file.name }}</div>
    <audio controls :src="getFileUrl(props.file)" class="w-full max-w-xs">
      浏览器不支持音频播放
    </audio>
  </div>
</template>
```

- [ ] **Step 2: Commit**

```bash
git -C "D:/Xiaoke/Works/Tools/XiaokeTools" add -A
git -C "D:/Xiaoke/Works/Tools/XiaokeTools" commit -m "feat(m4): AudioPlayer component"
```

---

## Task 5: FontPreview 组件（字体字形）

动态 @font-face 加载 ttf/otf，渲染样例文字。

**Files:**
- Create: `src/components/preview/FontPreview.vue`

- [ ] **Step 1: 创建 src/components/preview/FontPreview.vue**

```vue
<script setup lang="ts">
import { ref, watch, onUnmounted } from "vue";
import { getFileUrl } from "../../ipc/fileUrl";
import type { FileNode } from "../../types/library";

const props = defineProps<{ file: FileNode }>();
const fontFamily = ref("");
let styleEl: HTMLStyleElement | null = null;

const SAMPLE =
  "ABCDEFGHIJKLMNOPQRSTUVWXYZ abcdefghijklmnopqrstuvwxyz 0123456789";
const SAMPLE_CN = "游戏素材 字体预览 永和九年岁在癸丑";

watch(
  () => props.file,
  (f) => {
    // 清理旧样式
    if (styleEl) {
      document.head.removeChild(styleEl);
      styleEl = null;
    }
    const fam = `preview-font-${f.id}`;
    fontFamily.value = fam;
    const url = getFileUrl(f);
    styleEl = document.createElement("style");
    styleEl.textContent = `@font-face { font-family: "${fam}"; src: url("${url}"); }`;
    document.head.appendChild(styleEl);
  },
  { immediate: true },
);

onUnmounted(() => {
  if (styleEl) document.head.removeChild(styleEl);
});
</script>

<template>
  <div class="w-full flex flex-col gap-3 py-2">
    <div class="text-5xl text-center" :style="{ fontFamily: fontFamily }">Aa</div>
    <div class="text-sm text-slate-200 px-2" :style="{ fontFamily: fontFamily }">{{ SAMPLE }}</div>
    <div class="text-base text-slate-200 px-2" :style="{ fontFamily: fontFamily }">{{ SAMPLE_CN }}</div>
    <div class="text-xs text-slate-500 px-2">⚠ 字体可能不支持中文</div>
  </div>
</template>
```

- [ ] **Step 2: Commit**

```bash
git -C "D:/Xiaoke/Works/Tools/XiaokeTools" add -A
git -C "D:/Xiaoke/Works/Tools/XiaokeTools" commit -m "feat(m4): FontPreview component with dynamic @font-face"
```

---

## Task 6: SourcePlaceholder 组件（源文件占位）

xcf/psd/ase 等，显示文件信息 + 引导专业软件。

**Files:**
- Create: `src/components/preview/SourcePlaceholder.vue`

- [ ] **Step 1: 创建 src/components/preview/SourcePlaceholder.vue**

```vue
<script setup lang="ts">
import type { FileNode } from "../../types/library";

const props = defineProps<{ file: FileNode }>();

const SOFTWARE: Record<string, string> = {
  blend: "Blender",
  psd: "Photoshop",
  xcf: "GIMP",
  ase: "Aseprite",
  ai: "Illustrator",
};
</script>

<template>
  <div class="w-full flex flex-col items-center gap-2 py-6">
    <div class="text-5xl">⚙️</div>
    <div class="text-sm text-slate-300">源文件</div>
    <div class="text-xs text-slate-500 text-center px-4">
      {{ SOFTWARE[props.file.ext] ?? "专业" }} 格式，需用对应软件打开编辑
    </div>
  </div>
</template>
```

- [ ] **Step 2: Commit**

```bash
git -C "D:/Xiaoke/Works/Tools/XiaokeTools" add -A
git -C "D:/Xiaoke/Works/Tools/XiaokeTools" commit -m "feat(m4): SourcePlaceholder component"
```

---

## Task 7: PreviewPane 按 viewer 路由各预览器

右栏预览面板根据 viewer 选择组件。

**Files:**
- Modify: `src/components/PreviewPane.vue`

- [ ] **Step 1: PreviewPane 改为按 viewer 路由**

`<script setup>` 加 import：

```ts
import { viewerForKind } from "../utils/viewer";
import TextPreview from "./preview/TextPreview.vue";
import AudioPlayer from "./preview/AudioPlayer.vue";
import FontPreview from "./preview/FontPreview.vue";
import SourcePlaceholder from "./preview/SourcePlaceholder.vue";
```

加 computed：

```ts
const viewer = computed(() =>
  file.value ? viewerForKind(file.value.kind) : "fallback",
);
```

模板预览区域改为按 viewer 分发：

```html
<div class="flex-1 flex items-center justify-center bg-slate-900 min-h-48 overflow-auto">
  <img
    v-if="viewer === 'image' || viewer === 'animated' || viewer === 'vector'"
    :src="getFileUrl(file)"
    class="max-w-full max-h-96 object-contain"
  />
  <AudioPlayer v-else-if="viewer === 'audio'" :file="file" />
  <FontPreview v-else-if="viewer === 'font'" :file="file" />
  <TextPreview v-else-if="viewer === 'text'" :file="file" />
  <SourcePlaceholder v-else-if="viewer === 'binary-source'" :file="file" />
  <div v-else class="text-5xl">📦</div>
</div>
```

> 注意：`file` 可能为 null，模板里这些组件需在 `v-if="file"` 包裹下。PreviewPane 已有 `v-if="file"` 的外层，保留。

- [ ] **Step 2: 验证编译**

Run: `npm run build`
Expected: 通过。

- [ ] **Step 3: Commit**

```bash
git -C "D:/Xiaoke/Works/Tools/XiaokeTools" add -A
git -C "D:/Xiaoke/Works/Tools/XiaokeTools" commit -m "feat(m4): PreviewPane routes by viewer to typed preview components"
```

---

## Task 8: M4 端到端验收

- [ ] **Step 1: 启动应用**

Run: `npm run tauri dev`

- [ ] **Step 2: 准备**（添加库+扫描，如已有可跳过）

- [ ] **Step 3: 逐 viewer 验收**

每个找一个对应文件点开预览：

| viewer | 在哪找 | 验收点 |
|---|---|---|
| vector (svg) | 04_图标UI 里找 .svg | 右栏显示矢量图 |
| animated (gif) | 01_平台跳跃 里找 .gif | 右栏显示动画 |
| audio (ogg) | 06_特效粒子 或搜 ogg | 右栏音频播放控件 |
| font (ttf) | 04_图标UI/rpg-icons_font | 右栏显示字形样例 |
| text (xml/tmx/json) | 各包里的 .xml/.tmx | 右栏显示文本内容 |
| binary-source (xcf/psd/ase) | 12_源文件_PSD | 右栏显示源文件占位提示 |

- [ ] **Step 4: 核对验收标准**

M4 验收点（设计 §7）：
- [x] GIF 能播放（右栏 img 原生支持动画）
- [x] SVG 正确渲染
- [x] 音频可播放
- [x] 字体显示字形
- [x] 文本可查看内容

- [ ] **Step 5: Commit 验收**

```bash
git -C "D:/Xiaoke/Works/Tools/XiaokeTools" add -A
git -C "D:/Xiaoke/Works/Tools/XiaokeTools" commit -m "test(m4): e2e acceptance - multitype preview"
```

---

## M4 完成定义

- [ ] viewer 映射工具（kind→viewer）
- [ ] 缩略图墙按 viewer 显示（image/animated/vector 显示图，其他图标）
- [ ] 右栏预览按 viewer 路由：image/animated/vector/svg 直显、audio 播放、font 字形、text 内容、binary-source 占位
- [ ] 全部纯前端实现，无 Rust 改动
