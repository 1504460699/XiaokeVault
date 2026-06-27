<script setup lang="ts">
import { onMounted } from "vue";
import TopBar from "./components/TopBar.vue";
import CategoryTree from "./components/CategoryTree.vue";
import PackageGrid from "./components/PackageGrid.vue";
import PreviewPane from "./components/PreviewPane.vue";
import { useLibraryStore } from "./stores/libraryStore";

const store = useLibraryStore();
onMounted(async () => {
  await store.loadLibraries();
  if (store.currentLibId !== null) await store.loadCategories();
});
</script>

<template>
  <div class="h-full flex flex-col bg-slate-900 text-slate-100">
    <TopBar />
    <div class="flex-1 flex overflow-hidden">
      <CategoryTree />
      <PackageGrid />
      <PreviewPane />
    </div>
  </div>
</template>
