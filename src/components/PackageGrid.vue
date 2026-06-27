<script setup lang="ts">
import { storeToRefs } from "pinia";
import { useI18n } from "vue-i18n";
import { useLibraryStore } from "../stores/libraryStore";
import { useTreeStore } from "../stores/treeStore";
import { useSelectionStore } from "../stores/selectionStore";
import { useSearchStore } from "../stores/searchStore";
import FileGrid from "./FileGrid.vue";

const { t } = useI18n();

const store = useLibraryStore();
const treeStore = useTreeStore();
const sel = useSelectionStore();
const search = useSearchStore();
const { currentPkgId, packages } = storeToRefs(store);
const { currentDirId } = storeToRefs(treeStore);
const { pkgStates } = storeToRefs(sel);
const { locateFileId } = storeToRefs(search);

function onLocated() {
  search.consumeLocate();
}

function fmtBytes(b: number): string {
  if (b > 1e9) return (b / 1e9).toFixed(1) + " GB";
  if (b > 1e6) return (b / 1e6).toFixed(1) + " MB";
  if (b > 1e3) return (b / 1e3).toFixed(0) + " KB";
  return b + " B";
}

async function onTogglePkg(e: Event, pkgId: number) {
  e.stopPropagation();
  await sel.ensureProject();
  const isAll = pkgStates.value[pkgId] === "all";
  await sel.togglePackage(pkgId, isAll);
  if (store.currentCategoryId !== null) {
    await sel.refreshPkgStates(store.currentCategoryId);
  }
}
</script>

<template>
  <main class="flex-1 overflow-hidden flex flex-col">
    <!-- 树视图：选中目录时显示文件网格 -->
    <FileGrid
      v-if="treeStore.viewMode === 'tree' && currentDirId !== null"
      :locate-file-id="locateFileId"
      @located="onLocated"
    />
    <!-- 树视图未选目录时的占位 -->
    <div
      v-else-if="treeStore.viewMode === 'tree'"
      class="flex-1 flex items-center justify-center text-slate-500 text-sm"
    >
      {{ t("tree.selectDir") }}
    </div>
    <!-- 两级视图：包内文件 -->
    <FileGrid
      v-else-if="currentPkgId !== null"
      :locate-file-id="locateFileId"
      @located="onLocated"
    />
    <!-- 两级视图：包网格 -->
    <template v-else>
      <div
        class="px-4 py-2 text-sm text-slate-400 border-b border-slate-700 shrink-0"
      >
        {{ t("packageGrid.packageCount", { n: packages.length }) }}
      </div>
      <div
        class="flex-1 overflow-y-auto p-4 grid gap-3 content-start"
        style="grid-template-columns: repeat(auto-fill, minmax(220px, 1fr))"
      >
        <div
          v-for="pkg in packages"
          :key="pkg.id"
          class="bg-slate-800 rounded-lg p-3 cursor-pointer hover:bg-slate-700 border border-slate-700"
          @click="store.selectPackage(pkg.id)"
        >
          <div class="flex items-center">
            <input
              type="checkbox"
              class="mr-2 accent-sky-500"
              :checked="pkgStates[pkg.id] === 'all'"
              @click="onTogglePkg($event, pkg.id)"
            />
            <span class="font-medium text-sm truncate flex-1" :title="pkg.name">{{ pkg.name }}</span>
          </div>
          <div class="text-xs text-slate-400 mt-1">
            {{ pkg.file_count }} {{ t("common.files") }} · {{ fmtBytes(pkg.total_bytes) }}
          </div>
          <div v-if="pkg.has_zip" class="text-xs text-amber-400 mt-1">
            ⚠ {{ t("packageGrid.hasZip") }}
          </div>
          <div v-if="pkg.license" class="text-xs text-emerald-400 mt-1">
            {{ pkg.license }}
          </div>
        </div>
        <div
          v-if="packages.length === 0"
          class="col-span-full text-center text-slate-500 py-8"
        >
          {{ t("fileGrid.selectLeftCategory") }}
        </div>
      </div>
    </template>
  </main>
</template>
