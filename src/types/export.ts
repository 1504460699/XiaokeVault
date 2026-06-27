export interface ExportProgress {
  stage: "copy" | "zip" | "credits" | "manifest" | "done" | "error";
  done: number;
  total: number;
  current: string;
}

export interface ExportResult {
  output_path: string;
  file_count: number;
  total_bytes: number;
}
