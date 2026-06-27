<script setup lang="ts">
import { storeToRefs } from "pinia";
import { useLibraryStore } from "../stores/libraryStore";

const store = useLibraryStore();
const { categories, currentCategoryId } = storeToRefs(store);

function fmtBytes(b: number): string {
  if (b > 1e9) return (b / 1e9).toFixed(1) + " GB";
  if (b > 1e6) return (b / 1e6).toFixed(1) + " MB";
  if (b > 1e3) return (b / 1e3).toFixed(0) + " KB";
  return b + " B";
}
</script>

<template>
  <aside
    class="w-64 shrink-0 overflow-y-auto bg-slate-800/50 border-r border-slate-700"
  >
    <ul class="py-2">
      <li
        v-for="cat in categories"
        :key="cat.id"
        class="px-3 py-2 cursor-pointer hover:bg-slate-700/50"
        :class="
          cat.id === currentCategoryId ? 'bg-sky-600/30 border-l-2 border-sky-400' : ''
        "
        @click="store.selectCategory(cat.id)"
      >
        <div class="text-sm truncate">{{ cat.name }}</div>
        <div class="text-xs text-slate-400">
          {{ cat.package_count }} 包 · {{ cat.file_count }} 文件 ·
          {{ fmtBytes(cat.total_bytes) }}
        </div>
      </li>
      <li v-if="categories.length === 0" class="px-3 py-4 text-sm text-slate-500">
        无分类。请先扫描库。
      </li>
    </ul>
  </aside>
</template>
