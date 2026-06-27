import { convertFileSrc } from "@tauri-apps/api/core";
import type { FileNode } from "../types/library";

// convertFileSrc 把本地文件路径转成 webview 可加载的 asset: URL。
// abs_path 形如 "D:/Xiaoke/.../x.png"，统一用 / 分隔。
export function getFileUrl(f: FileNode): string {
  return convertFileSrc(f.abs_path);
}
