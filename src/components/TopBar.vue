<script setup lang="ts">
import { storeToRefs } from "pinia";
import { open } from "@tauri-apps/plugin-dialog";
import { useLibraryStore } from "../stores/libraryStore";
import { useSearchStore } from "../stores/searchStore";
import WindowControls from "./WindowControls.vue";

defineEmits<{ dedup: []; types: [] }>();

const store = useLibraryStore();
const { libraries, currentLibId, scanning, scanReport, error } = storeToRefs(store);
const search = useSearchStore();

async function onScan() {
  await store.scanCurrent();
}

async function onAddLibrary() {
  // 原生文件夹选择对话框
  const selected = await open({ directory: true, multiple: false, title: "选择素材库根目录" });
  if (!selected || Array.isArray(selected)) return;
  const rootPath = selected;
  // 默认库名 = 目录名，用户可改
  const defaultName = rootPath.replace(/\\/g, "/").split("/").filter(Boolean).pop() ?? "GameAssets";
  const name = window.prompt("输入库名称：", defaultName);
  if (!name) return;
  try {
    await store.addLibrary(name, rootPath);
    await store.loadCategories();
  } catch (e: unknown) {
    alert("添加失败：" + String(e));
  }
}

function onLibChange(e: Event) {
  const id = Number((e.target as HTMLSelectElement).value);
  store.selectLibrary(id);
}
</script>

<template>
  <header
    class="flex items-center gap-3 px-4 h-12 bg-slate-800 border-b border-slate-700 shrink-0"
    data-tauri-drag-region
  >
    <span class="font-bold text-sky-400">XiaokeTools</span>
    <select
      class="bg-slate-700 text-slate-100 px-2 py-1 rounded text-sm"
      :value="currentLibId ?? ''"
      @change="onLibChange"
    >
      <option value="" disabled>选择库…</option>
      <option v-for="lib in libraries" :key="lib.id" :value="lib.id">
        {{ lib.name }}
      </option>
    </select>
    <button
      class="px-3 py-1 rounded bg-sky-600 hover:bg-sky-500 disabled:opacity-50 text-sm"
      :disabled="scanning || currentLibId === null"
      @click="onScan"
    >
      {{ scanning ? "扫描中…" : "扫描" }}
    </button>
    <button
      class="px-3 py-1 rounded bg-amber-700 hover:bg-amber-600 disabled:opacity-50 text-sm"
      :disabled="currentLibId === null"
      @click="$emit('dedup')"
    >
      去重
    </button>
    <button
      class="px-3 py-1 rounded bg-slate-700 hover:bg-slate-600 text-sm"
      @click="$emit('types')"
    >
      类型
    </button>
    <input
      v-model="search.query"
      type="text"
      placeholder="全局搜索文件名…"
      class="bg-slate-700 text-slate-100 px-2 py-1 rounded text-sm w-48"
      @keyup.enter="search.run()"
    />
    <button
      class="px-3 py-1 rounded bg-slate-700 hover:bg-slate-600 text-sm"
      @click="onAddLibrary"
    >
      + 添加库
    </button>
    <div class="ml-auto text-xs text-slate-400 flex items-center gap-2">
      <span v-if="scanReport"
        >上次扫描：{{ scanReport.total_files }} 文件 /
        {{ Math.round(scanReport.duration_ms / 1000) }}s</span
      >
      <button
        v-if="scanReport && scanReport.unknown_extensions.length > 0"
        class="px-2 py-0.5 rounded bg-amber-700 hover:bg-amber-600 text-amber-50"
        :title="scanReport.unknown_extensions.map((e) => '.' + e[0] + '×' + e[1]).join(' ')"
        @click="$emit('types')"
      >
        发现 {{ scanReport.unknown_extensions.length }} 种未知类型 → 管理类型
      </button>
      <span v-if="error" class="text-red-400">⚠ {{ error }}</span>
    </div>
    <WindowControls />
  </header>
</template>
