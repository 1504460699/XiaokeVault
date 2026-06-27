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
    return invoke<void>("upsert_asset_type", t);
  },
  async remove(kind: string): Promise<void> {
    return invoke<void>("delete_asset_type", { kind });
  },
  async reclassify(): Promise<{ updated: number }> {
    return invoke<{ updated: number }>("reclassify_all");
  },
};
