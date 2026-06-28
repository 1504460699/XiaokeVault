import { invoke } from "@tauri-apps/api/core";
import type {
  Library,
  FileNode,
  ScanReport,
  Project,
  SelectionSummary,
  SearchHit,
  DirNode,
} from "../types/library";

// 命令名与 src-tauri/src/lib.rs generate_handler 注册一致
export const ipc = {
  async listLibraries(): Promise<Library[]> {
    return invoke<Library[]>("list_libraries");
  },
  async addLibrary(name: string, rootPath: string): Promise<Library> {
    return invoke<Library>("add_library", { name, rootPath });
  },
  async scanLibraryFull(libId: number): Promise<ScanReport> {
    return invoke<ScanReport>("scan_library_full", { libId });
  },
  async needsRescan(libId: number): Promise<boolean> {
    return invoke<boolean>("needs_rescan", { libId });
  },
  async searchFiles(
    query: string,
    kind: string | null,
  ): Promise<SearchHit[]> {
    return invoke<SearchHit[]>("search_files", { query, kind });
  },

  // 目录树
  async getDirectoryTree(libId: number): Promise<DirNode[]> {
    return invoke<DirNode[]>("get_directory_tree", { libId });
  },
  async getDirectoryFiles(directoryId: number): Promise<FileNode[]> {
    return invoke<FileNode[]>("get_directory_files", { directoryId });
  },
  async getSubtreeFiles(directoryId: number): Promise<FileNode[]> {
    return invoke<FileNode[]>("get_subtree_files", { directoryId });
  },
  async getAllLibraryFiles(libraryId: number): Promise<FileNode[]> {
    return invoke<FileNode[]>("get_all_library_files", { libraryId });
  },

  // 项目与勾选
  async createProject(name: string, exportRoot: string): Promise<Project> {
    return invoke<Project>("create_project", { name, exportRoot });
  },
  async listProjects(): Promise<Project[]> {
    return invoke<Project[]>("list_projects");
  },
  async setSelection(
    projectId: number,
    scope: "directory" | "file" | "exclude",
    directoryId: number | null,
    fileId: number | null,
    action: "add" | "remove",
  ): Promise<void> {
    return invoke<void>("set_selection", {
      projectId,
      scope,
      directoryId,
      fileId,
      action,
    });
  },
  async getDirectorySelectionState(
    projectId: number,
    directoryId: number,
  ): Promise<"all" | "partial" | "none"> {
    return invoke<"all" | "partial" | "none">("get_directory_selection_state", {
      projectId,
      directoryId,
    });
  },
  async getSelectionSummary(projectId: number): Promise<SelectionSummary> {
    return invoke<SelectionSummary>("get_selection_summary", { projectId });
  },
  async clearSelections(projectId: number): Promise<void> {
    return invoke<void>("clear_selections", { projectId });
  },
  async getSelectedFileIds(projectId: number, dirId: number): Promise<number[]> {
    return invoke<number[]>("get_selected_file_ids", { projectId, dirId });
  },
};
