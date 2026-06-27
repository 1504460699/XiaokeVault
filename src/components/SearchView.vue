<script setup lang="ts">
import { storeToRefs } from "pinia";
import { useI18n } from "vue-i18n";
import { useSearchStore } from "../stores/searchStore";
import { useLibraryStore } from "../stores/libraryStore";
import { useSelectionStore } from "../stores/selectionStore";
import { viewerForKind, iconForViewer, canShowThumb } from "../utils/viewer";
import { getFileUrl } from "../ipc/fileUrl";
import type { SearchHit } from "../types/library";

const { t } = useI18n();
const search = useSearchStore();
const lib = useLibraryStore();
const sel = useSelectionStore();
const { results, searching } = storeToRefs(search);

function fmtBytes(b: number): string {
  if (b > 1e6) return (b / 1e6).toFixed(1) + " MB";
  if (b > 1e3) return (b / 1e3).toFixed(0) + " KB";
  return b + " B";
}

// 点击结果：定位到该文件所在包并预览（保持搜索状态，便于返回继续浏览结果）
async function locate(h: SearchHit) {
  await lib.selectPackage(h.package_id);
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

    <div v-if="searching" class="flex-1 flex items-center justify-center text-slate-500 text-sm">
      {{ t("search.searching") }}
    </div>

    <div v-else-if="results.length === 0" class="flex-1 flex items-center justify-center text-slate-500 text-sm">
      {{ t("search.noMatch") }}
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
