<script setup lang="ts">
import { onMounted, ref, watch } from "vue";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useI18n } from "vue-i18n";
import TopBar from "./components/TopBar.vue";
import DirectoryTree from "./components/DirectoryTree.vue";
import FileGrid from "./components/FileGrid.vue";
import PreviewPane from "./components/PreviewPane.vue";
import SearchView from "./components/SearchView.vue";
import ExportDialog from "./components/ExportDialog.vue";
import TypeSettings from "./components/TypeSettings.vue";
import Toast from "./components/Toast.vue";
import { ipc } from "./ipc/library";
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
const showTypes = ref(false);

onMounted(async () => {
  await syncWindowTitle();
  await store.loadLibraries();
  if (store.currentLibId !== null) {
    // 加载目录树数据
    await treeStore.loadTree(store.currentLibId);

    // 迁移后自动重扫：0006 迁移清理了旧两级文件，需重新扫描填充 directory_id。
    // 检测到需要重扫时静默触发（用户会看到扫描进度，但无需手动操作）。
    try {
      const needRescan = await ipc.needsRescan(store.currentLibId);
      if (needRescan) {
        console.info("[迁移] 检测到需要重扫，自动触发扫描以重建索引…");
        await store.scanCurrent();
      }
    } catch (e) {
      console.warn("[迁移] 自动重扫检查失败：", e);
    }
  }
  await selStore.loadProjects();
  await selStore.refreshSummary();

  // 监听自动增量扫描完成事件，刷新目录树
  await listen("library://auto-scanned", async () => {
    store.autoScanning = false;
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

// 手动扫描完成（scanReport 变化）后刷新目录树
watch(
  () => store.scanReport,
  async () => {
    if (store.currentLibId !== null) {
      await treeStore.loadTree(store.currentLibId);
    }
  },
);

// 进目录时：回填已勾选文件 + 刷新该目录勾选状态
watch(
  () => treeStore.currentDirId,
  async (dirId) => {
    if (dirId !== null && dirId >= 0) {
      await selStore.loadFileSelections(dirId);
      await selStore.refreshDirState(dirId);
    }
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
    <TopBar @types="showTypes = true" />
    <div class="flex-1 flex overflow-hidden">
      <DirectoryTree />
      <SearchView v-if="searchStore.active && treeStore.currentDirId === null" />
      <FileGrid
        v-else
        :locate-file-id="searchStore.locateFileId"
        @located="searchStore.consumeLocate()"
      />
      <PreviewPane @export="onOpenExport" />
    </div>
    <ExportDialog :show="showExport" @close="showExport = false" />
    <TypeSettings :show="showTypes" @close="showTypes = false" />
    <Toast />
  </div>
</template>
