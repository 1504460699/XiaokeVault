import { invoke } from "@tauri-apps/api/core";
import type { DupGroup, DedupReport } from "../types/dedup";

export const dedupIpc = {
  async runDedup(libId: number): Promise<DedupReport> {
    return invoke<DedupReport>("run_dedup", { libId });
  },
  async getGroups(): Promise<DupGroup[]> {
    return invoke<DupGroup[]>("get_duplicate_groups");
  },
  async removeDuplicate(fileId: number): Promise<string> {
    return invoke<string>("remove_duplicate", { fileId });
  },
};
