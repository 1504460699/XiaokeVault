// 与 src-tauri/src/library.rs 的 Serialize 结构对齐
export interface Library {
  id: number;
  name: string;
  root_path: string;
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

export interface SelectionSummary {
  directory_count: number;
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
  directory_id: number | null;
  /** 文件所在目录的相对路径（库根下的路径，用于结果展示） */
  directory_path: string;
}

// 目录树节点（递归，对应 src-tauri/src/tree.rs 的 DirNode）
export interface DirNode {
  id: number;
  name: string;
  path: string;
  depth: number;
  file_count: number;
  total_bytes: number;
  children: DirNode[];
}
