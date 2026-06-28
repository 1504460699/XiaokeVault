<script setup lang="ts">
import { storeToRefs } from "pinia";
import { useI18n } from "vue-i18n";
import { useSearchStore } from "../stores/searchStore";
import { useLibraryStore } from "../stores/libraryStore";
import { useTreeStore } from "../stores/treeStore";
import { useSelectionStore } from "../stores/selectionStore";
import { viewerForKind, iconForViewer, canShowThumb } from "../utils/viewer";
import { getFileUrl } from "../ipc/fileUrl";
import type { SearchHit } from "../types/library";

const { t } = useI18n();
const search = useSearchStore();
const lib = useLibraryStore();
const tree = useTreeStore();
const sel = useSelectionStore();
const { results, searching, hasSearched, query } = storeToRefs(search);

function fmtBytes(b: number): string {
  if (b > 1e6) return (b / 1e6).toFixed(1) + " MB";
  if (b > 1e3) return (b / 1e3).toFixed(0) + " KB";
  return b + " B";
}

// 点击结果：定位到该文件所在目录/包并预览（保持搜索状态，便于返回继续浏览结果）
async function locate(h: SearchHit) {
  if (tree.viewMode === "tree" && h.directory_id !== null) {
    // 树视图：定位到目录
    await tree.selectDirectory(h.directory_id);
  } else if (h.package_id > 0) {
    // 两级视图：定位到包
    await lib.selectPackage(h.package_id);
  }
  sel.setPreview(h.id);
  search.requestLocate(h.id);
  // 不 close：保持搜索 active，FileGrid 渲染后用户可点“返回搜索”回到结果
}
</script>

<template>
  <div class="flex-1 flex flex-col overflow-hidden">
    <div class="px-4 py-2 border-b border-slate-700 shrink-0 flex items-center gap-2">
      <span class="text-sm text-slate-300">{{ t("search.globalResults") }}</span>
      <span class="text-xs text-slate-500">{{ t("search.matchCount", { n: results.length }) }}</span>
      <button class="ml-auto text-sky-400 hover:underline text-sm" @click="search.close()">
        ← {{ t("search.backToResults") }}
      </button>
    </div>

    <div v-if="searching" class="flex-1 flex flex-col items-center justify-center text-slate-500 text-sm gap-3">
      <svg class="animate-spin h-8 w-8 text-sky-400" viewBox="0 0 24 24" fill="none">
        <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4" />
        <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z" />
      </svg>
      <span>{{ t("search.searching") }}</span>
    </div>

    <div
      v-else-if="hasSearched && results.length === 0"
      class="flex-1 flex flex-col items-center justify-center text-slate-500 text-sm gap-2"
    >
      <div class="text-4xl opacity-50">🔍</div>
      <span>{{ t("search.noMatch") }}</span>
      <span class="text-xs text-slate-600">“{{ query }}”</span>
    </div>

    <div v-else-if="!hasSearched" class="flex-1 flex flex-col items-center justify-center text-slate-500 text-sm gap-2">
      <div class="text-4xl opacity-50">🔍</div>
      <span>{{ t("search.typeToSearch") }}</span>
    </div>

    <div v-else class="flex-1 overflow-auto p-2">
      <div class="grid gap-2 content-start" style="grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));">
        <div
          v-for="h in results"
          :key="h.id"
          class="bg-slate-800 rounded border border-slate-700 p-2 flex flex-col cursor-pointer hover:border-sky-500"
          @click="locate(h)"
        >
          <div class="h-20 flex items-center justify-center bg-slate-900 rounded mb-1 overflow-hidden">
            <img
              v-if="canShowThumb(viewerForKind(h.kind))"
              :src="getFileUrl(h as any)"
              class="max-w-full max-h-full object-contain"
              loading="lazy"
            />
            <div v-else class="text-3xl">{{ iconForViewer(viewerForKind(h.kind)) }}</div>
          </div>
          <div class="text-xs text-slate-200 truncate" :title="h.name">{{ h.name }}</div>
          <div class="text-xs text-slate-500 truncate" :title="`${h.category_name} / ${h.package_name}`">{{ h.category_name }} / {{ h.package_name }}</div>
          <div class="flex items-center justify-between mt-1">
            <span class="text-xs text-slate-500">{{ fmtBytes(h.bytes) }} · {{ h.kind }}</span>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
