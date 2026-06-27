import { defineStore } from "pinia";
import { ref } from "vue";
import { ipc } from "../ipc/library";
import type { SearchHit } from "../types/library";

export const useSearchStore = defineStore("search", () => {
  const query = ref("");
  const kind = ref(""); // 空=全部
  const results = ref<SearchHit[]>([]);
  const searching = ref(false);
  const active = ref(false); // 是否显示搜索视图
  // 定位请求：设置后 FileGrid 滚动到该文件，然后清空
  const locateFileId = ref<number | null>(null);

  async function run() {
    const q = query.value.trim();
    if (!q) {
      results.value = [];
      active.value = false;
      return;
    }
    active.value = true;
    searching.value = true;
    try {
      results.value = await ipc.searchFiles(q, kind.value || null);
    } finally {
      searching.value = false;
    }
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
    query.value = "";
    kind.value = "";
    results.value = [];
    active.value = false;
  }

  return { query, kind, results, searching, active, locateFileId, run, requestLocate, consumeLocate, close, clear };
});
