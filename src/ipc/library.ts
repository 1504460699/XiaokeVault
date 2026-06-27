import { invoke } from "@tauri-apps/api/core";
import type {
  Library,
  Category,
  PackageSummary,
  FileNode,
  ScanReport,
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
};
