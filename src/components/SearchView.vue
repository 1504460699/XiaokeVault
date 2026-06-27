<script setup lang="ts">
import { storeToRefs } from "pinia";
import { useSearchStore } from "../stores/searchStore";
import { useLibraryStore } from "../stores/libraryStore";
import { viewerForKind, iconForViewer, canShowThumb } from "../utils/viewer";
import { getFileUrl } from "../ipc/fileUrl";
import type { SearchHit } from "../types/library";

const search = useSearchStore();
const lib = useLibraryStore();
const { results, searching } = storeToRefs(search);

function fmtBytes(b: number): string {
  if (b > 1e6) return (b / 1e6).toFixed(1) + " MB";
  if (b > 1e3) return (b / 1e3).toFixed(0) + " KB";
  return b + " B";
}

// 点击结果：预览该文件。把 hit 构造为 FileNode 形状供 PreviewPane
function onHit(h: SearchHit) {
  // 设置临时预览：通过 selectPackage 进入该包并预览
  // 简化：直接设置 preview，但 PreviewPane 从 files 找——这里用 convert
  // 实际跳转到所在包最一致，但会重新加载。这里先做预览提示。
  alert(`文件：${h.name}\n分类：${h.category_name}\n包：${h.package_name}\n\n点击「定位」按钮可跳转到该包。`);
}

async function locate(h: SearchHit) {
  // 跳转到该文件所在包（需要先选分类再选包）
  await lib.selectPackage(h.package_id);
  search.close();
}
</script>

<template>
  <div class="flex-1 flex flex-col overflow-hidden">
    <div class="px-4 py-2 border-b border-slate-700 shrink-0 flex items-center gap-2">
      <span class="text-sm text-slate-300">全局搜索结果</span>
      <span class="text-xs text-slate-500">{{ results.length }} 个匹配（最多 500）</span>
      <button class="ml-auto text-sky-400 hover:underline text-sm" @click="search.close()">
        ← 返回浏览
      </button>
    </div>

    <div v-if="searching" class="flex-1 flex items-center justify-center text-slate-500 text-sm">
      搜索中…
    </div>

    <div v-else-if="results.length === 0" class="flex-1 flex items-center justify-center text-slate-500 text-sm">
      无匹配结果
    </div>

    <div v-else class="flex-1 overflow-auto p-2">
      <div class="grid gap-2 content-start" style="grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));">
        <div
          v-for="h in results"
          :key="h.id"
          class="bg-slate-800 rounded border border-slate-700 p-2 flex flex-col cursor-pointer hover:border-sky-500"
          @click="onHit(h)"
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
          <div class="text-xs text-slate-500 truncate">{{ h.category_name }} / {{ h.package_name }}</div>
          <div class="flex items-center justify-between mt-1">
            <span class="text-xs text-slate-500">{{ fmtBytes(h.bytes) }} · {{ h.kind }}</span>
            <button
              class="text-xs text-sky-400 hover:underline"
              @click.stop="locate(h)"
            >
              定位
            </button>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
