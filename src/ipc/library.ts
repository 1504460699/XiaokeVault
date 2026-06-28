import { invoke } from "@tauri-apps/api/core";
import type {
  Library,
  Category,
  PackageSummary,
  FileNode,
  ScanReport,
  Project,
  PackageSelectionState,
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
  async getCategories(libId: number): Promise<Category[]> {
    return invoke<Category[]>("get_categories", { libId });
  },
  async getPackages(categoryId: number): Promise<PackageSummary[]> {
    return invoke<PackageSummary[]>("get_packages", { categoryId });
  },
  async getPackageFiles(pkgId: number): Promise<FileNode[]> {
    return invoke<FileNode[]>("get_package_files", { pkgId });
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
    scope: "package" | "file" | "exclude",
    packageId: number | null,
    fileId: number | null,
    action: "add" | "remove",
  ): Promise<void> {
    return invoke<void>("set_selection", {
      projectId,
      scope,
      packageId,
      fileId,
      action,
    });
  },
  async getCategorySelectionStates(
    projectId: number,
    categoryId: number,
  ): Promise<PackageSelectionState[]> {
    return invoke<PackageSelectionState[]>("get_category_selection_states", {
      projectId,
      categoryId,
    });
  },
  async getSelectionSummary(projectId: number): Promise<SelectionSummary> {
    return invoke<SelectionSummary>("get_selection_summary", { projectId });
  },
  async clearSelections(projectId: number): Promise<void> {
    return invoke<void>("clear_selections", { projectId });
  },
  async getSelectedFileIds(projectId: number, pkgId: number): Promise<number[]> {
    return invoke<number[]>("get_selected_file_ids", { projectId, pkgId });
  },
};
