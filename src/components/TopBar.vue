<script setup lang="ts">
import { storeToRefs } from "pinia";
import { useI18n } from "vue-i18n";
import { open } from "@tauri-apps/plugin-dialog";
import { useLibraryStore } from "../stores/libraryStore";
import { useSearchStore } from "../stores/searchStore";
import { setLocale } from "../i18n";
import WindowControls from "./WindowControls.vue";

defineEmits<{ dedup: []; types: [] }>();

const store = useLibraryStore();
const { libraries, currentLibId, scanning, autoScanning, scanReport, error } = storeToRefs(store);
const search = useSearchStore();
const { t, locale } = useI18n();

async function onScan() {
  await store.scanCurrent();
}

async function onAddLibrary() {
  // 原生文件夹选择对话框
  const selected = await open({ directory: true, multiple: false, title: t("topbar.addLibrary") });
  if (!selected || Array.isArray(selected)) return;
  const rootPath = selected;
  // 默认库名 = 目录名，用户可改
  const defaultName = rootPath.replace(/\\/g, "/").split("/").filter(Boolean).pop() ?? "GameAssets";
  const name = window.prompt(t("types.name"), defaultName);
  if (!name) return;
  try {
    await store.addLibrary(name, rootPath);
    await store.loadCategories();
  } catch (e: unknown) {
    alert(e instanceof Error ? e.message : String(e));
  }
}

function onLibChange(e: Event) {
  const id = Number((e.target as HTMLSelectElement).value);
  store.selectLibrary(id);
}

function onLangChange(e: Event) {
  const val = (e.target as HTMLSelectElement).value as "zh" | "en";
  setLocale(val);
}
</script>

<template>
  <header
    class="flex items-center gap-3 px-4 h-12 bg-slate-800 border-b border-slate-700 shrink-0"
    data-tauri-drag-region
  >
    <span class="font-bold text-sky-400">{{ t("brand.name") }}</span>
    <select
      class="bg-slate-700 text-slate-100 px-2 py-1 rounded text-sm"
      :value="currentLibId ?? ''"
      @change="onLibChange"
    >
      <option value="" disabled>{{ t("topbar.selectLibrary") }}</option>
      <option v-for="lib in libraries" :key="lib.id" :value="lib.id">
        {{ lib.name }}
      </option>
    </select>
    <button
      class="px-3 py-1 rounded bg-sky-600 hover:bg-sky-500 disabled:opacity-50 text-sm"
      :disabled="scanning || currentLibId === null"
      @click="onScan"
    >
      {{ scanning ? t("topbar.scanning") : t("topbar.scan") }}
    </button>
    <button
      class="px-3 py-1 rounded bg-amber-700 hover:bg-amber-600 disabled:opacity-50 text-sm"
      :disabled="currentLibId === null"
      @click="$emit('dedup')"
    >
      {{ t("topbar.dedup") }}
    </button>
    <button
      class="px-3 py-1 rounded bg-slate-700 hover:bg-slate-600 text-sm"
      @click="$emit('types')"
    >
      {{ t("topbar.types") }}
    </button>
    <input
      v-model="search.query"
      type="text"
      :placeholder="t('topbar.searchPlaceholder')"
      class="bg-slate-700 text-slate-100 px-2 py-1 rounded text-sm w-48"
      @keyup.enter="search.run()"
    />
    <button
      class="px-3 py-1 rounded bg-slate-700 hover:bg-slate-600 text-sm"
      @click="onAddLibrary"
    >
      {{ t("topbar.addLibrary") }}
    </button>
    <div class="ml-auto text-xs text-slate-400 flex items-center gap-2">
      <span v-if="autoScanning" class="text-sky-400 animate-pulse">{{ t("topbar.autoScanning") }}</span>
      <select
        class="bg-slate-700 text-slate-100 px-1 py-0.5 rounded text-xs"
        :value="locale"
        @change="onLangChange"
        :title="t('topbar.language')"
      >
        <option value="zh">中文</option>
        <option value="en">English</option>
      </select>
      <span v-if="scanReport"
        >{{ t("topbar.lastScan") }}{{ scanReport.total_files }} {{ t("common.file") }} /
        {{ Math.round(scanReport.duration_ms / 1000) }}s</span
      >
      <button
        v-if="scanReport && scanReport.unknown_extensions.length > 0"
        class="px-2 py-0.5 rounded bg-amber-700 hover:bg-amber-600 text-amber-50"
        :title="scanReport.unknown_extensions.map((e) => '.' + e[0] + '×' + e[1]).join(' ')"
        @click="$emit('types')"
      >
        {{ t("topbar.foundUnknown", { n: scanReport.unknown_extensions.length }) }}
      </button>
      <span v-if="error" class="text-red-400">⚠ {{ error }}</span>
    </div>
    <WindowControls />
  </header>
</template>
