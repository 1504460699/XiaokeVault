<script setup lang="ts">
import { onMounted, ref, watch } from "vue";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useI18n } from "vue-i18n";
import TopBar from "./components/TopBar.vue";
import CategoryTree from "./components/CategoryTree.vue";
import DirectoryTree from "./components/DirectoryTree.vue";
import PackageGrid from "./components/PackageGrid.vue";
import PreviewPane from "./components/PreviewPane.vue";
import SearchView from "./components/SearchView.vue";
import ExportDialog from "./components/ExportDialog.vue";
import DedupPanel from "./components/DedupPanel.vue";
import TypeSettings from "./components/TypeSettings.vue";
import { useLibraryStore } from "./stores/libraryStore";
import { useSelectionStore } from "./stores/selectionStore";
import { useSearchStore } from "./stores/searchStore";
import { useTreeStore } from "./stores/treeStore";

const store = useLibraryStore();
const selStore = useSelectionStore();
const searchStore = useSearchStore();
const treeStore = useTreeStore();
const { t, locale } = useI18n();
const showExport = ref(false);

// 窗口标题跟随语言：中文=笑客宝库，英文=XiaokeVault
async function syncWindowTitle() {
  try {
    await getCurrentWindow().setTitle(t("brand.name"));
  } catch {
    // 非 tauri 环境（如纯 web 预览）忽略
  }
}
const showDedup = ref(false);
const showTypes = ref(false);

onMounted(async () => {
  await syncWindowTitle();
  await store.loadLibraries();
  if (store.currentLibId !== null) {
    await store.loadCategories();
    // 加载目录树数据
    await treeStore.loadTree(store.currentLibId);
  }
  await selStore.loadProjects();
  if (store.currentCategoryId !== null) {
    await selStore.refreshPkgStates(store.currentCategoryId);
  }
  await selStore.refreshSummary();

  // 监听自动增量扫描完成事件，刷新分类/包/文件列表
  await listen("library://auto-scanned", async () => {
    store.autoScanning = false;
    await store.loadCategories();
    if (store.currentCategoryId !== null) {
      await store.loadPackages();
    }
    // 若当前在某个包内，则刷新该包文件列表，使新增文件立即可见
    if (store.currentPkgId !== null) {
      await store.selectPackage(store.currentPkgId);
    }
    // 同步刷新目录树
    if (store.currentLibId !== null) {
      await treeStore.loadTree(store.currentLibId);
      if (treeStore.currentDirId !== null) {
        await treeStore.selectDirectory(treeStore.currentDirId);
      }
    }
  });

  // 监听自动扫描开始事件（仅用于状态提示）
  await listen("library://auto-scanning", () => {
    store.autoScanning = true;
  });
});

// 语言切换时同步窗口标题
watch(locale, () => {
  syncWindowTitle();
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
      <CategoryTree v-if="treeStore.viewMode === 'category'" />
      <DirectoryTree v-else />
      <SearchView v-if="searchStore.active && store.currentPkgId === null && treeStore.currentDirId === null" />
      <PackageGrid v-else />
      <PreviewPane @export="onOpenExport" />
    </div>
    <ExportDialog :show="showExport" @close="showExport = false" />
    <DedupPanel :show="showDedup" @close="showDedup = false" />
    <TypeSettings :show="showTypes" @close="showTypes = false" />
  </div>
</template>
