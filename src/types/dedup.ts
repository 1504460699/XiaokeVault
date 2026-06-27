export interface DupMember {
  file_id: number | null;
  package_id: number | null;
  package_name: string | null;
  rel_path: string;
  role: "keep" | "remove";
}

export interface DupGroup {
  id: number;
  reason: string;
  detail: string | null;
  members: DupMember[];
}

export interface DedupReport {
  groups: number;
  removable_files: number;
  removable_bytes: number;
}
