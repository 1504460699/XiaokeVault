import { invoke } from "@tauri-apps/api/core";
import type { ExportResult } from "../types/export";

export const exportIpc = {
  async runExport(
    projectId: number,
    format: "folder" | "zip",
    exportRoot: string,
  ): Promise<ExportResult> {
    return invoke<ExportResult>("run_export", { projectId, format, exportRoot });
  },
};
