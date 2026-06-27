import { defineStore } from "pinia";
import { ref } from "vue";
import { ipc } from "../ipc/library";
import type { Project, SelectionSummary } from "../types/library";

export const useSelectionStore = defineStore("selection", () => {
  const currentProjectId = ref<number | null>(null);
  const projects = ref<Project[]>([]);
  // packageId -> state（当前分类下）
  const pkgStates = ref<Record<number, string>>({});
  // 当前包内已选文件 ID（文件级勾选）
  const selectedFileIds = ref<Set<number>>(new Set());
  // 选中预览的文件
  const previewFileId = ref<number | null>(null);
  const summary = ref<SelectionSummary>({
    package_count: 0,
    file_count: 0,
    total_bytes: 0,
  });

  async function loadProjects() {
    projects.value = await ipc.listProjects();
    if (currentProjectId.value === null && projects.value.length > 0) {
      currentProjectId.value = projects.value[0].id;
    }
  }

  async function createProject(name: string, exportRoot: string) {
    const p = await ipc.createProject(name, exportRoot);
    projects.value.unshift(p);
    currentProjectId.value = p.id;
    return p;
  }

  function selectProject(id: number) {
    currentProjectId.value = id;
  }

  /// 确保存在一个当前项目；没有则创建默认项目
  async function ensureProject() {
    if (currentProjectId.value !== null) return;
    if (projects.value.length === 0) {
      await createProject("默认项目", "");
    } else {
      currentProjectId.value = projects.value[0].id;
    }
  }

  function setPreview(id: number | null) {
    previewFileId.value = id;
  }

  /// 切换整包勾选
  async function togglePackage(pkgId: number, currentlyAll: boolean) {
    if (currentProjectId.value === null) return;
    const action = currentlyAll ? "remove" : "add";
    await ipc.setSelection(currentProjectId.value, "package", pkgId, null, action);
    await refreshSummary();
  }

  /// 切换单文件勾选
  async function toggleFile(fileId: number, currentlySelected: boolean) {
    if (currentProjectId.value === null) return;
    const action = currentlySelected ? "remove" : "add";
    await ipc.setSelection(currentProjectId.value, "file", null, fileId, action);
    if (currentlySelected) selectedFileIds.value.delete(fileId);
    else selectedFileIds.value.add(fileId);
    selectedFileIds.value = new Set(selectedFileIds.value);
    await refreshSummary();
  }

  /// 刷新某分类下包状态
  async function refreshPkgStates(categoryId: number) {
    if (currentProjectId.value === null) {
      pkgStates.value = {};
      return;
    }
    const states = await ipc.getCategorySelectionStates(
      currentProjectId.value,
      categoryId,
    );
    const m: Record<number, string> = {};
    for (const s of states) m[s.package_id] = s.state;
    pkgStates.value = m;
  }

  /// 进入包时从 DB 回填已勾选的文件 ID（持久化读取）
  async function loadFileSelections(pkgId: number) {
    if (currentProjectId.value === null) {
      selectedFileIds.value = new Set();
      return;
    }
    const ids = await ipc.getSelectedFileIds(currentProjectId.value, pkgId);
    selectedFileIds.value = new Set(ids);
  }

  /// 清空当前项目的所有勾选
  async function clearAll() {
    if (currentProjectId.value === null) return;
    await ipc.clearSelections(currentProjectId.value);
    pkgStates.value = {};
    selectedFileIds.value = new Set();
    await refreshSummary();
  }

  async function refreshSummary() {
    if (currentProjectId.value === null) return;
    summary.value = await ipc.getSelectionSummary(currentProjectId.value);
  }

  return {
    currentProjectId,
    projects,
    pkgStates,
    selectedFileIds,
    previewFileId,
    summary,
    loadProjects,
    createProject,
    ensureProject,
    selectProject,
    setPreview,
    togglePackage,
    toggleFile,
    clearAll,
    refreshPkgStates,
    loadFileSelections,
    refreshSummary,
  };
});
