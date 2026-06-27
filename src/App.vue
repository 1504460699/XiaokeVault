<script setup lang="ts">
import { onMounted, ref, watch } from "vue";
import TopBar from "./components/TopBar.vue";
import CategoryTree from "./components/CategoryTree.vue";
import PackageGrid from "./components/PackageGrid.vue";
import PreviewPane from "./components/PreviewPane.vue";
import ExportDialog from "./components/ExportDialog.vue";
import { useLibraryStore } from "./stores/libraryStore";
import { useSelectionStore } from "./stores/selectionStore";

const store = useLibraryStore();
const selStore = useSelectionStore();
const showExport = ref(false);

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

// 进包时重置文件级勾选集合
watch(
  () => store.currentPkgId,
  () => {
    selStore.resetFileSelections();
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
    <TopBar />
    <div class="flex-1 flex overflow-hidden">
      <CategoryTree />
      <PackageGrid />
      <PreviewPane @export="onOpenExport" />
    </div>
    <ExportDialog :show="showExport" @close="showExport = false" />
  </div>
</template>
