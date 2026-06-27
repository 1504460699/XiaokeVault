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

  function close() {
    active.value = false;
  }

  function clear() {
    query.value = "";
    kind.value = "";
    results.value = [];
    active.value = false;
  }

  return { query, kind, results, searching, active, run, close, clear };
});
