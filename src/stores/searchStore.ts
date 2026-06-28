import { defineStore } from "pinia";
import { ref } from "vue";
import { ipc } from "../ipc/library";
import { useLibraryStore } from "./libraryStore";
import { useTreeStore } from "./treeStore";
import type { SearchHit } from "../types/library";

export const useSearchStore = defineStore("search", () => {
  const query = ref("");
  const kind = ref(""); // 空=全部
  const results = ref<SearchHit[]>([]);
  const searching = ref(false);
  const active = ref(false); // 是否显示搜索视图
  const hasSearched = ref(false); // 是否已发起过一次搜索（用于区分“未搜索”与“搜索后0结果”）
  // 定位请求：设置后 FileGrid 滚动到该文件，然后清空
  const locateFileId = ref<number | null>(null);

  // 防抖句柄：实时搜索时合并连续输入
  let debounceTimer: ReturnType<typeof setTimeout> | null = null;

  // 把当前位置（包/目录）让出去，确保 SearchView 的 v-if 三条件同时成立
  function releaseCurrentLocation() {
    const lib = useLibraryStore();
    const tree = useTreeStore();
    if (lib.currentPkgId !== null) lib.backToPackages();
    if (tree.currentDirId !== null) tree.clearFiles();
  }

  async function run() {
    const q = query.value.trim();
    if (!q) {
      results.value = [];
      active.value = false;
      hasSearched.value = false;
      return;
    }
    // 搜索时强制接管中间区：清空包/目录选中，否则 SearchView 不会渲染
    releaseCurrentLocation();
    active.value = true;
    searching.value = true;
    hasSearched.value = true;
    try {
      results.value = await ipc.searchFiles(q, kind.value || null);
    } finally {
      searching.value = false;
    }
  }

  // 实时搜索入口：500ms 防抖。连续输入只发最后一次请求；空查询立即清空。
  function runDebounced(delay = 500) {
    if (debounceTimer) clearTimeout(debounceTimer);
    const q = query.value.trim();
    if (!q) {
      // 清空立即生效，无需等待
      results.value = [];
      active.value = false;
      hasSearched.value = false;
      return;
    }
    debounceTimer = setTimeout(() => {
      void run();
    }, delay);
  }

  // 请求定位某文件（设置 id，FileGrid 监听后滚动）
  function requestLocate(fileId: number) {
    locateFileId.value = fileId;
  }

  // FileGrid 滚动完成后调用，清空定位请求
  function consumeLocate() {
    locateFileId.value = null;
  }

  function close() {
    active.value = false;
  }

  function clear() {
    if (debounceTimer) clearTimeout(debounceTimer);
    query.value = "";
    kind.value = "";
    results.value = [];
    active.value = false;
    hasSearched.value = false;
  }

  return {
    query,
    kind,
    results,
    searching,
    active,
    hasSearched,
    locateFileId,
    run,
    runDebounced,
    requestLocate,
    consumeLocate,
    close,
    clear,
  };
});
