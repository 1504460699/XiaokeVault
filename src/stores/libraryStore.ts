import { defineStore } from "pinia";
import { ref, computed } from "vue";
import { ipc } from "../ipc/library";
import type {
  Library,
  Category,
  PackageSummary,
  FileNode,
  ScanReport,
} from "../types/library";

export const useLibraryStore = defineStore("library", () => {
  const libraries = ref<Library[]>([]);
  const currentLibId = ref<number | null>(null);
  const categories = ref<Category[]>([]);
  const currentCategoryId = ref<number | null>(null);
  const packages = ref<PackageSummary[]>([]);
  const currentPkgId = ref<number | null>(null);
  const files = ref<FileNode[]>([]);
  const scanning = ref(false);
  const scanReport = ref<ScanReport | null>(null);
  const error = ref<string | null>(null);

  const currentCategory = computed(
    () => categories.value.find((c) => c.id === currentCategoryId.value) ?? null,
  );
  const currentPackage = computed(
    () => packages.value.find((p) => p.id === currentPkgId.value) ?? null,
  );

  async function loadLibraries() {
    libraries.value = await ipc.listLibraries();
    if (currentLibId.value === null && libraries.value.length > 0) {
      currentLibId.value = libraries.value[0].id;
    }
  }

  async function addLibrary(name: string, rootPath: string) {
    const lib = await ipc.addLibrary(name, rootPath);
    libraries.value.push(lib);
    currentLibId.value = lib.id;
  }

  async function selectLibrary(libId: number) {
    currentLibId.value = libId;
    currentCategoryId.value = null;
    currentPkgId.value = null;
    packages.value = [];
    files.value = [];
    await loadCategories();
  }

  async function loadCategories() {
    if (currentLibId.value === null) return;
    categories.value = await ipc.getCategories(currentLibId.value);
    if (currentCategoryId.value === null && categories.value.length > 0) {
      currentCategoryId.value = categories.value[0].id;
    }
  }

  async function selectCategory(catId: number) {
    currentCategoryId.value = catId;
    currentPkgId.value = null;
    files.value = [];
    await loadPackages();
  }

  async function loadPackages() {
    if (currentCategoryId.value === null) return;
    packages.value = await ipc.getPackages(currentCategoryId.value);
  }

  async function selectPackage(pkgId: number) {
    currentPkgId.value = pkgId;
    files.value = await ipc.getPackageFiles(pkgId);
  }

  async function backToPackages() {
    currentPkgId.value = null;
    files.value = [];
  }

  async function scanCurrent() {
    if (currentLibId.value === null) return;
    scanning.value = true;
    error.value = null;
    try {
      scanReport.value = await ipc.scanLibraryFull(currentLibId.value);
      await loadCategories();
      if (currentCategoryId.value !== null) await loadPackages();
    } catch (e: unknown) {
      error.value = String(e);
    } finally {
      scanning.value = false;
    }
  }

  return {
    libraries,
    currentLibId,
    categories,
    currentCategoryId,
    currentCategory,
    packages,
    currentPkgId,
    currentPackage,
    files,
    scanning,
    scanReport,
    error,
    loadLibraries,
    addLibrary,
    selectLibrary,
    loadCategories,
    selectCategory,
    loadPackages,
    selectPackage,
    backToPackages,
    scanCurrent,
  };
});
