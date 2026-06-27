<script setup lang="ts">
import { storeToRefs } from "pinia";
import { useI18n } from "vue-i18n";
import { useLibraryStore } from "../stores/libraryStore";
import { useTreeStore } from "../stores/treeStore";

const { t } = useI18n();
const store = useLibraryStore();
const treeStore = useTreeStore();
const { categories, currentCategoryId } = storeToRefs(store);
const { viewMode } = storeToRefs(treeStore);

function fmtBytes(b: number): string {
  if (b > 1e9) return (b / 1e9).toFixed(1) + " GB";
  if (b > 1e6) return (b / 1e6).toFixed(1) + " MB";
  if (b > 1e3) return (b / 1e3).toFixed(0) + " KB";
  return b + " B";
}
</script>

<template>
  <div class="flex flex-col w-64 shrink-0">
    <!-- 视图切换栏 -->
    <div class="flex gap-1 p-2 bg-slate-800 border-b border-slate-700 shrink-0">
      <button
        class="flex-1 px-2 py-1 rounded text-xs"
        :class="viewMode === 'category' ? 'bg-sky-600 text-white' : 'bg-slate-700 text-slate-300 hover:bg-slate-600'"
        @click="treeStore.setViewMode('category')"
      >📁 {{ t("tree.viewCategory") }}</button>
      <button
        class="flex-1 px-2 py-1 rounded text-xs"
        :class="viewMode === 'tree' ? 'bg-sky-600 text-white' : 'bg-slate-700 text-slate-300 hover:bg-slate-600'"
        @click="treeStore.setViewMode('tree')"
      >🌳 {{ t("tree.viewTree") }}</button>
    </div>
    <!-- 原有分类列表 -->
    <aside class="flex-1 overflow-y-auto bg-slate-800/50 border-r border-slate-700">
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
          <div class="text-sm truncate" :title="cat.name">{{ cat.name }}</div>
          <div class="text-xs text-slate-400">
            {{ t("category.pkgCount", { n: cat.package_count, f: cat.file_count, b: fmtBytes(cat.total_bytes) }) }}
          </div>
        </li>
        <li v-if="categories.length === 0" class="px-3 py-4 text-sm text-slate-500">
          {{ t("category.noCategory") }}
        </li>
      </ul>
    </aside>
  </div>
</template>
