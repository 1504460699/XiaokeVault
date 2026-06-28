import { defineStore } from "pinia";
import { ref } from "vue";
import { ipc } from "../ipc/library";
import { handleError } from "../utils/toast";
import type { Library, ScanReport } from "../types/library";

export const useLibraryStore = defineStore("library", () => {
  const libraries = ref<Library[]>([]);
  const currentLibId = ref<number | null>(null);
  const scanning = ref(false);
  const autoScanning = ref(false);
  const scanReport = ref<ScanReport | null>(null);
  const error = ref<string | null>(null);

  async function loadLibraries() {
    try {
      libraries.value = await ipc.listLibraries();
      if (currentLibId.value === null && libraries.value.length > 0) {
        currentLibId.value = libraries.value[0].id;
      }
    } catch (e) {
      handleError(e, "加载库列表失败");
    }
  }

  async function addLibrary(name: string, rootPath: string) {
    const lib = await ipc.addLibrary(name, rootPath);
    libraries.value.push(lib);
    currentLibId.value = lib.id;
  }

  async function selectLibrary(libId: number) {
    currentLibId.value = libId;
  }

  async function scanCurrent() {
    if (currentLibId.value === null) return;
    scanning.value = true;
    error.value = null;
    try {
      scanReport.value = await ipc.scanLibraryFull(currentLibId.value);
    } catch (e: unknown) {
      handleError(e, "扫描失败");
    } finally {
      scanning.value = false;
    }
  }

  return {
    libraries,
    currentLibId,
    scanning,
    autoScanning,
    scanReport,
    error,
    loadLibraries,
    addLibrary,
    selectLibrary,
    scanCurrent,
  };
});
