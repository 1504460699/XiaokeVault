import { defineStore } from "pinia";
import { ref } from "vue";
import { ipc } from "../ipc/library";
import type { DirNode, FileNode } from "../types/library";

export const useTreeStore = defineStore("tree", () => {
  const tree = ref<DirNode[]>([]);
  const currentDirId = ref<number | null>(null);
  const files = ref<FileNode[]>([]);

  // 左侧视图模式：'category' | 'tree'，记忆到 localStorage
  const viewMode = ref<"category" | "tree">(
    (localStorage.getItem("vault.leftView") as "category" | "tree") || "category",
  );

  function setViewMode(mode: "category" | "tree") {
    viewMode.value = mode;
    localStorage.setItem("vault.leftView", mode);
  }

  async function loadTree(libId: number) {
    tree.value = await ipc.getDirectoryTree(libId);
  }

  async function selectDirectory(dirId: number) {
    currentDirId.value = dirId;
    files.value = await ipc.getDirectoryFiles(dirId);
  }

  function clearFiles() {
    files.value = [];
    currentDirId.value = null;
  }

  return {
    tree,
    currentDirId,
    files,
    viewMode,
    setViewMode,
    loadTree,
    selectDirectory,
    clearFiles,
  };
});
