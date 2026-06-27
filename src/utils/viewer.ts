// kind → viewer 映射（与设计 §3.2 内置表 viewer 列对齐）
const VIEWER_BY_KIND: Record<string, string> = {
  image: "image",
  animated: "animated",
  vector: "vector",
  audio: "audio",
  font: "font",
  text: "text",
  model3d: "3d",
  source_blend: "binary-source",
  source_pixel: "binary-source",
  source_design: "binary-source",
  archive: "fallback",
  legacy_media: "fallback",
  other: "fallback",
};

export function viewerForKind(kind: string): string {
  return VIEWER_BY_KIND[kind] ?? "fallback";
}

// viewer → 占位图标（emoji，缩略图墙用）
const ICON_BY_VIEWER: Record<string, string> = {
  audio: "🎵",
  font: "🔤",
  text: "📄",
  "3d": "🧊",
  "binary-source": "⚙️",
  fallback: "📦",
};

export function iconForViewer(viewer: string): string {
  return ICON_BY_VIEWER[viewer] ?? "📦";
}

// 缩略图墙里能直接用 img 显示的 viewer
export function canShowThumb(viewer: string): boolean {
  return viewer === "image" || viewer === "animated" || viewer === "vector";
}
