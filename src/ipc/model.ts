import { invoke } from "@tauri-apps/api/core";

export interface ModelPath {
  path: string;
  source: "blender" | "error";
  message: string;
}

export async function getModelGlb(blendPath: string): Promise<ModelPath> {
  return invoke<ModelPath>("get_model_glb", { blendPath });
}
