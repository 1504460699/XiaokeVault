# M7 类型管理 UI 实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:subagent-driven-development or superpowers:executing-plans.

**Goal:** 完成设计里"资源类型可扩充"的闭环——设置页可新增/编辑/删除自定义类型、把扫描发现的未知扩展名一键转为新类型、修改后重新分类全库。让未来下载新格式资源时不改代码就能管理。

**Architecture:** 后端 asset_types.rs 加 CRUD command（list/upsert/delete/reclassify），写入 asset_types 表覆盖内置默认。前端新增 TypeSettings 面板（设置入口），列表展示类型 + 编辑扩展名/预览器 + 未知扩展名提示。typesStore 维护注册表。

**Tech Stack:** Rust（sqlx）, Vue 3 + Pinia

**Spec:** `docs/superpowers/specs/2026-06-27-game-asset-manager-design.md` §3.2 可配置类型注册表, §4.3 list_asset_types/upsert_asset_type/delete_asset_type/reclassify_all, §5.5 设置页

**前置条件:** M1（asset_types 表 + Registry）已完成。当前内置表已覆盖库内 99.8%。

---

## 关键决策

- 内置类型不可删，只能被覆盖（asset_types 表 built_in=1 表示覆盖内置项）
- 用户新增项 built_in=0，可删
- list_asset_types 返回合并后的全表（内置默认 + 数据库覆盖）
- reclassify_all 按新注册表重算全库 files.kind（增量扫描时的 ON CONFLICT 已会更新，但 reclassify 不依赖文件变化，强制全量重算）
- 扫描报告的 unknown_extensions 驱动"未知类型提示"

---

## 文件结构

```
src-tauri/src/
├── asset_types.rs          # 加 list/upsert/delete/reclassify command
└── lib.rs                  # 注册 command
src/
├── types/library.ts        # AssetType 已有
├── ipc/types.ts            # 类型管理 ipc
├── stores/typesStore.ts    # 注册表状态
└── components/
    └── TypeSettings.vue    # 设置页（模态）
```

---

## Task 1: 后端类型注册表 CRUD + 重新分类

**Files:**
- Modify: `src-tauri/src/asset_types.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: asset_types.rs 末尾加 command**

```rust
use tauri::State;
use sqlx::SqlitePool;

/// 列出合并后的全表（内置默认 + 数据库覆盖）
#[tauri::command]
pub async fn list_asset_types(pool: State<'_, SqlitePool>) -> Result<Vec<AssetType>, String> {
    let reg = Registry::load(&pool).await.map_err(|e| e.to_string())?;
    Ok(reg.all().to_vec())
}

/// 新增/编辑类型（覆盖内置或追加自定义）
#[tauri::command]
pub async fn upsert_asset_type(
    kind: String,
    label: String,
    extensions: Vec<String>,
    viewer: String,
    is_source: bool,
    built_in: bool,
    pool: State<'_, SqlitePool>,
) -> Result<(), String> {
    let exts_json = serde_json::to_string(&extensions).map_err(|e| e.to_string())?;
    sqlx::query(
        "INSERT INTO asset_types(kind,label,extensions,viewer,is_source,built_in)
         VALUES(?,?,?,?,?,?)
         ON CONFLICT(kind) DO UPDATE SET label=excluded.label, extensions=excluded.extensions,
           viewer=excluded.viewer, is_source=excluded.is_source",
    )
    .bind(&kind).bind(&label).bind(&exts_json).bind(&viewer)
    .bind(if is_source { 1 } else { 0 }).bind(if built_in { 1 } else { 0 })
    .execute(&*pool).await.map_err(|e| e.to_string())?;
    Ok(())
}

/// 删除类型（仅限用户新增项 built_in=0）
#[tauri::command]
pub async fn delete_asset_type(kind: String, pool: State<'_, SqlitePool>) -> Result<(), String> {
    let res = sqlx::query("DELETE FROM asset_types WHERE kind=? AND built_in=0")
        .bind(&kind).execute(&*pool).await.map_err(|e| e.to_string())?;
    if res.rows_affected() == 0 {
        return Err("内置类型不可删除（只能覆盖编辑）".into());
    }
    Ok(())
}

/// 按当前注册表重新分类全库（重算 files.kind）
#[derive(serde::Serialize)]
pub struct ReclassifyReport {
    pub updated: i64,
}

#[tauri::command]
pub async fn reclassify_all(pool: State<'_, SqlitePool>) -> Result<ReclassifyReport, String> {
    let reg = Registry::load(&pool).await.map_err(|e| e.to_string())?;
    // 取所有未删除文件
    let files: Vec<(i64, String)> = sqlx::query_as("SELECT id, ext FROM files WHERE deleted=0")
        .fetch_all(&*pool).await.map_err(|e| e.to_string())?;
    let mut updated = 0i64;
    for (id, ext) in files {
        let new_kind = reg.kind_for(&ext);
        let res = sqlx::query("UPDATE files SET kind=? WHERE id=? AND kind!=?")
            .bind(new_kind).bind(id).bind(new_kind)
            .execute(&*pool).await.map_err(|e| e.to_string())?;
        if res.rows_affected() > 0 {
            updated += 1;
        }
    }
    Ok(ReclassifyReport { updated })
}
```

- [ ] **Step 2: lib.rs 注册 command**

```rust
asset_types::list_asset_types,
asset_types::upsert_asset_type,
asset_types::delete_asset_type,
asset_types::reclassify_all,
```

- [ ] **Step 3: 验证编译**

Run: `cd src-tauri && cargo build`

- [ ] **Step 4: Commit**

```bash
git -C "D:/Xiaoke/Works/Tools/XiaokeTools" add -A
git -C "D:/Xiaoke/Works/Tools/XiaokeTools" commit -m "feat(m7): asset type CRUD + reclassify_all commands"
```

---

## Task 2: 前端类型 IPC + store

**Files:**
- Create: `src/ipc/types.ts`
- Create: `src/stores/typesStore.ts`

- [ ] **Step 1: 创建 src/ipc/types.ts**

```ts
import { invoke } from "@tauri-apps/api/core";
import type { AssetType } from "../types/library";

export const typesIpc = {
  async list(): Promise<AssetType[]> {
    return invoke<AssetType[]>("list_asset_types");
  },
  async upsert(t: {
    kind: string; label: string; extensions: string[];
    viewer: string; is_source: boolean; built_in: boolean;
  }): Promise<void> {
    return invoke<void>("upsert_asset_type", t);
  },
  async remove(kind: string): Promise<void> {
    return invoke<void>("delete_asset_type", { kind });
  },
  async reclassify(): Promise<{ updated: number }> {
    return invoke<{ updated: number }>("reclassify_all");
  },
};
```

- [ ] **Step 2: 创建 src/stores/typesStore.ts**

```ts
import { defineStore } from "pinia";
import { ref } from "vue";
import { typesIpc } from "../ipc/types";
import type { AssetType } from "../types/library";

export const useTypesStore = defineStore("types", () => {
  const types = ref<AssetType[]>([]);
  const loading = ref(false);

  async function load() {
    loading.value = true;
    try {
      types.value = await typesIpc.list();
    } finally {
      loading.value = false;
    }
  }

  async function upsert(t: {
    kind: string; label: string; extensions: string[];
    viewer: string; is_source: boolean; built_in: boolean;
  }) {
    await typesIpc.upsert(t);
    await load();
  }

  async function remove(kind: string) {
    await typesIpc.remove(kind);
    await load();
  }

  async function reclassify() {
    return await typesIpc.reclassify();
  }

  return { types, loading, load, upsert, remove, reclassify };
});
```

- [ ] **Step 3: 验证编译**

Run: `npm run build`

- [ ] **Step 4: Commit**

```bash
git -C "D:/Xiaoke/Works/Tools/XiaokeTools" add -A
git -C "D:/Xiaoke/Works/Tools/XiaokeTools" commit -m "feat(m7): types ipc and store"
```

---

## Task 3: TypeSettings 设置面板

**Files:**
- Create: `src/components/TypeSettings.vue`
- Modify: `src/components/TopBar.vue`（加设置入口）
- Modify: `src/App.vue`（接入面板）

- [ ] **Step 1: 创建 src/components/TypeSettings.vue**

```vue
<script setup lang="ts">
import { ref, onMounted } from "vue";
import { storeToRefs } from "pinia";
import { useTypesStore } from "../stores/typesStore";
import type { AssetType } from "../types/library";

const props = defineProps<{ show: boolean }>();
const emit = defineEmits<{ close: [] }>();

const store = useTypesStore();
const { types, loading } = storeToRefs(store);

const editing = ref<AssetType | null>(null);
const extInput = ref("");
const reclassifyMsg = ref("");

const VIEWERS = ["image", "animated", "vector", "audio", "font", "text", "3d", "binary-source", "fallback"];

onMounted(() => store.load());

function startEdit(t: AssetType) {
  editing.value = { ...t };
  extInput.value = t.extensions.join(", ");
}

function startNew() {
  editing.value = {
    kind: "", label: "", extensions: [], viewer: "fallback", icon: null, is_source: false,
  };
  extInput.value = "";
}

async function save() {
  if (!editing.value) return;
  if (!editing.value.kind || !editing.value.label) {
    alert("kind 和显示名必填");
    return;
  }
  const isBuiltin = types.value.some((t) => t.kind === editing.value!.kind);
  const exts = extInput.value.split(",").map((s) => s.trim().toLowerCase()).filter(Boolean);
  await store.upsert({
    kind: editing.value.kind,
    label: editing.value.label,
    extensions: exts,
    viewer: editing.value.viewer,
    is_source: editing.value.is_source,
    built_in: isBuiltin,
  });
  editing.value = null;
}

async function remove(kind: string) {
  if (!confirm(`删除类型 ${kind}？`)) return;
  try {
    await store.remove(kind);
  } catch (e) {
    alert(String(e));
  }
}

async function onReclassify() {
  const r = await store.reclassify();
  reclassifyMsg.value = `已重新分类 ${r.updated} 个文件`;
}
</script>

<template>
  <Teleport to="body">
    <div v-if="props.show" class="fixed inset-0 bg-black/60 flex items-center justify-center z-50" @click.self="emit('close')">
      <div class="bg-slate-800 rounded-lg p-6 w-[680px] max-h-[85vh] flex flex-col text-slate-100">
        <div class="flex items-center mb-3">
          <h2 class="text-lg font-bold flex-1">资源类型管理</h2>
          <button class="px-3 py-1 rounded bg-sky-600 hover:bg-sky-500 text-sm" @click="startNew">+ 新增类型</button>
        </div>

        <!-- 编辑区 -->
        <div v-if="editing" class="bg-slate-900 rounded p-3 mb-3 border border-slate-700 space-y-2">
          <div class="flex gap-2">
            <input v-model="editing.kind" placeholder="类型标识(如 video)" class="flex-1 bg-slate-700 rounded px-2 py-1 text-sm" :disabled="types.some((t) => t.kind === editing!.kind)" />
            <input v-model="editing.label" placeholder="显示名(如 视频)" class="flex-1 bg-slate-700 rounded px-2 py-1 text-sm" />
          </div>
          <input v-model="extInput" placeholder="扩展名，逗号分隔(如 webm,mp4)" class="w-full bg-slate-700 rounded px-2 py-1 text-sm" />
          <div class="flex items-center gap-2 text-sm">
            <span class="text-slate-400">预览器：</span>
            <select v-model="editing.viewer" class="bg-slate-700 rounded px-2 py-1 text-sm">
              <option v-for="v in VIEWERS" :key="v" :value="v">{{ v }}</option>
            </select>
            <label class="flex items-center gap-1 ml-2">
              <input type="checkbox" v-model="editing.is_source" /> 源文件
            </label>
          </div>
          <div class="flex gap-2">
            <button class="px-3 py-1 rounded bg-emerald-600 hover:bg-emerald-500 text-sm" @click="save">保存</button>
            <button class="px-3 py-1 rounded bg-slate-600 hover:bg-slate-500 text-sm" @click="editing = null">取消</button>
          </div>
        </div>

        <!-- 类型列表 -->
        <div class="flex-1 overflow-auto">
          <table class="w-full text-sm">
            <thead class="text-xs text-slate-400 sticky top-0 bg-slate-800">
              <tr>
                <th class="text-left p-1">标识</th>
                <th class="text-left p-1">显示名</th>
                <th class="text-left p-1">扩展名</th>
                <th class="text-left p-1">预览器</th>
                <th class="text-left p-1">操作</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="t in types" :key="t.kind" class="border-t border-slate-700">
                <td class="p-1 font-mono text-sky-300">{{ t.kind }}</td>
                <td class="p-1">{{ t.label }}</td>
                <td class="p-1 text-xs text-slate-400">{{ t.extensions.join(", ") || "—" }}</td>
                <td class="p-1 text-xs">{{ t.viewer }}</td>
                <td class="p-1 whitespace-nowrap">
                  <button class="px-2 py-0.5 rounded bg-slate-600 hover:bg-slate-500 text-xs mr-1" @click="startEdit(t)">编辑</button>
                  <button class="px-2 py-0.5 rounded bg-red-700 hover:bg-red-600 text-xs" @click="remove(t.kind)">删</button>
                </td>
              </tr>
            </tbody>
          </table>
          <div v-if="loading" class="text-center text-slate-500 py-4 text-sm">加载中…</div>
        </div>

        <!-- 重新分类 -->
        <div class="flex items-center gap-3 mt-3 pt-3 border-t border-slate-700">
          <button class="px-3 py-1 rounded bg-amber-700 hover:bg-amber-600 text-sm" @click="onReclassify">
            按新类型重新分类全库
          </button>
          <span v-if="reclassifyMsg" class="text-xs text-emerald-400">{{ reclassifyMsg }}</span>
          <span class="text-xs text-slate-500 ml-auto">修改类型后需重新分类才生效</span>
        </div>

        <div class="flex justify-end mt-3">
          <button class="px-4 py-1 rounded bg-slate-600 hover:bg-slate-500 text-sm" @click="emit('close')">关闭</button>
        </div>
      </div>
    </div>
  </Teleport>
</template>
```

- [ ] **Step 2: TopBar 加「类型」按钮**

TopBar.vue 加 `defineEmits<{ dedup: []; types: [] }>()`（替换原 dedup emit），模板加：

```html
<button class="px-3 py-1 rounded bg-slate-700 hover:bg-slate-600 text-sm" @click="$emit('types')">类型</button>
```

- [ ] **Step 3: App.vue 接入 TypeSettings**

```ts
import TypeSettings from "./components/TypeSettings.vue";
const showTypes = ref(false);
```

模板：

```html
<TopBar @dedup="showDedup = true" @types="showTypes = true" />
...
<TypeSettings :show="showTypes" @close="showTypes = false" />
```

- [ ] **Step 4: 验证编译**

Run: `npm run build`

- [ ] **Step 5: Commit**

```bash
git -C "D:/Xiaoke/Works/Tools/XiaokeTools" add -A
git -C "D:/Xiaoke/Works/Tools/XiaokeTools" commit -m "feat(m7): TypeSettings panel + TopBar entry"
```

---

## Task 4: M7 端到端验收

- [ ] **Step 1: 启动应用** `npm run tauri dev`

- [ ] **Step 2: 浏览类型**

点顶栏「类型」→ 显示全部类型表（13 内置 + 自定义）。

- [ ] **Step 3: 编辑内置类型**

编辑某个内置类型（如 text），加一个扩展名（如 `ini`）→ 保存 → 点「重新分类」→ 报告"N 个文件已更新"。

- [ ] **Step 4: 新增自定义类型**

点「+ 新增类型」→ kind=video, 显示名=视频, 扩展名=webm,mp4, 预览器=fallback → 保存 → 列表出现 video。

- [ ] **Step 5: 删除自定义类型**

删除 video → 成功。尝试删除内置 image → 报错"内置类型不可删除"。

- [ ] **Step 6: 验收点**

- [x] 设置页可新增/编辑/删除自定义类型
- [x] 内置类型只能编辑不能删
- [x] 修改后重新分类生效

- [ ] **Step 7: Commit**

```bash
git -C "D:/Xiaoke/Works/Tools/XiaokeTools" add -A
git -C "D:/Xiaoke/Works/Tools/XiaokeTools" commit -m "test(m7): e2e acceptance - type management"
```

---

## M7 完成定义

- [ ] 类型注册表 CRUD command（list/upsert/delete/reclassify）
- [ ] TypeSettings 面板（表格 + 新增/编辑/删除 + 重新分类）
- [ ] 内置类型不可删、可覆盖编辑
- [ ] 修改后重新分类全库生效
