import { defineStore } from "pinia";
import { ref } from "vue";
import { ipc } from "../ipc/library";
import { handleError } from "../utils/toast";
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
    try {
      tree.value = await ipc.getDirectoryTree(libId);
    } catch (e) {
      handleError(e, "加载目录树失败");
    }
  }

  // 选中库根（虚拟节点 id=-1）：显示整库所有文件
  async function selectLibraryRoot(libId: number) {
    currentDirId.value = -1;
    try {
      files.value = await ipc.getAllLibraryFiles(libId);
    } catch (e) {
      handleError(e, "加载文件列表失败");
    }
  }

  async function selectDirectory(dirId: number) {
    currentDirId.value = dirId;
    try {
      // 递归取该目录及所有子目录的文件（点中间文件夹也能看到全部内容）
      files.value = await ipc.getSubtreeFiles(dirId);
    } catch (e) {
      handleError(e, "加载文件列表失败");
    }
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
    selectLibraryRoot,
    selectDirectory,
    clearFiles,
  };
});
