// 与 src-tauri/src/library.rs 的 Serialize 结构对齐
export interface Library {
  id: number;
  name: string;
  root_path: string;
}

export interface Category {
  id: number;
  name: string;
  sort_order: number;
  package_count: number;
  file_count: number;
  total_bytes: number;
}

export interface PackageSummary {
  id: number;
  name: string;
  path: string;
  file_count: number;
  total_bytes: number;
  has_zip: boolean;
  license: string | null;
}

export interface FileNode {
  id: number;
  rel_path: string;
  name: string;
  ext: string;
  kind: string;
  bytes: number;
  abs_path: string;
}

export interface ScanReport {
  new: number;
  updated: number;
  deleted: number;
  total_files: number;
  duration_ms: number;
  errors: string[];
  unknown_extensions: [string, number][];
}

// Rust 的 asset_types::AssetType
export interface AssetType {
  kind: string;
  label: string;
  extensions: string[];
  viewer: string;
  icon: string | null;
  is_source: boolean;
}

export interface Project {
  id: number;
  name: string;
  export_root: string;
}

export interface PackageSelectionState {
  package_id: number;
  state: "all" | "partial" | "excluded" | "none";
  file_count: number;
  selected_files: number;
}

export interface SelectionSummary {
  package_count: number;
  file_count: number;
  total_bytes: number;
}

export interface SearchHit {
  id: number;
  name: string;
  ext: string;
  kind: string;
  bytes: number;
  abs_path: string;
  package_name: string;
  category_name: string;
  package_id: number;
}
