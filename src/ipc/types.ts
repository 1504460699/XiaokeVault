import { invoke } from "@tauri-apps/api/core";
import type { AssetType } from "../types/library";

export const typesIpc = {
  async list(): Promise<AssetType[]> {
    return invoke<AssetType[]>("list_asset_types");
  },
  async upsert(t: {
    kind: string;
    label: string;
    extensions: string[];
    viewer: string;
    is_source: boolean;
    built_in: boolean;
  }): Promise<void> {
    // Tauri 2 默认把 Rust snake_case 转 camelCase，前端须传 camelCase
    return invoke<void>("upsert_asset_type", {
      kind: t.kind,
      label: t.label,
      extensions: t.extensions,
      viewer: t.viewer,
      isSource: t.is_source,
      builtIn: t.built_in,
    });
  },
  async remove(kind: string): Promise<void> {
    return invoke<void>("delete_asset_type", { kind });
  },
  async reclassify(): Promise<{ updated: number }> {
    return invoke<{ updated: number }>("reclassify_all");
  },
};
