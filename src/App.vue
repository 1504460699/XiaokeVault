<script setup lang="ts">
import { onMounted, ref, watch } from "vue";
import TopBar from "./components/TopBar.vue";
import CategoryTree from "./components/CategoryTree.vue";
import PackageGrid from "./components/PackageGrid.vue";
import PreviewPane from "./components/PreviewPane.vue";
import SearchView from "./components/SearchView.vue";
import ExportDialog from "./components/ExportDialog.vue";
import DedupPanel from "./components/DedupPanel.vue";
import TypeSettings from "./components/TypeSettings.vue";
import { useLibraryStore } from "./stores/libraryStore";
import { useSelectionStore } from "./stores/selectionStore";
import { useSearchStore } from "./stores/searchStore";

const store = useLibraryStore();
const selStore = useSelectionStore();
const searchStore = useSearchStore();
const showExport = ref(false);
const showDedup = ref(false);
const showTypes = ref(false);

onMounted(async () => {
  await store.loadLibraries();
  if (store.currentLibId !== null) await store.loadCategories();
  await selStore.loadProjects();
  if (store.currentCategoryId !== null) {
    await selStore.refreshPkgStates(store.currentCategoryId);
  }
  await selStore.refreshSummary();
});

// 切换分类时刷新该分类的勾选状态
watch(
  () => store.currentCategoryId,
  async (catId) => {
    if (catId !== null) await selStore.refreshPkgStates(catId);
  },
);

// 进包时从 DB 回填已勾选的文件（持久化读取）
watch(
  () => store.currentPkgId,
  async (pkgId) => {
    if (pkgId !== null) await selStore.loadFileSelections(pkgId);
  },
);

async function onOpenExport() {
  if (selStore.currentProjectId === null) {
    await selStore.createProject("默认项目", "");
  }
  await selStore.refreshSummary();
  showExport.value = true;
}
</script>

<template>
  <div class="h-full flex flex-col bg-slate-900 text-slate-100">
    <TopBar @dedup="showDedup = true" @types="showTypes = true" />
    <div class="flex-1 flex overflow-hidden">
      <CategoryTree />
      <SearchView v-if="searchStore.active" />
      <PackageGrid v-else />
      <PreviewPane @export="onOpenExport" />
    </div>
    <ExportDialog :show="showExport" @close="showExport = false" />
    <DedupPanel :show="showDedup" @close="showDedup = false" />
    <TypeSettings :show="showTypes" @close="showTypes = false" />
  </div>
</template>
