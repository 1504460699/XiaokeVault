<script setup lang="ts">
import { storeToRefs } from "pinia";
import { useSelectionStore } from "../stores/selectionStore";

const sel = useSelectionStore();
const { summary, currentProjectId, projects } = storeToRefs(sel);

defineEmits<{ export: [] }>();

function fmtBytes(b: number): string {
  if (b > 1e9) return (b / 1e9).toFixed(2) + " GB";
  if (b > 1e6) return (b / 1e6).toFixed(1) + " MB";
  if (b > 1e3) return (b / 1e3).toFixed(0) + " KB";
  return b + " B";
}
</script>

<template>
  <div class="px-3 py-2 border-b border-slate-700 text-sm space-y-1">
    <div class="text-slate-300 text-xs">
      {{ currentProjectId !== null ? "项目：" + (projects.find((p) => p.id === currentProjectId)?.name ?? "—") : "未创建项目" }}
    </div>
    <div class="text-slate-400 text-xs">
      {{ summary.package_count }} 包 · {{ summary.file_count }} 文件 · {{ fmtBytes(summary.total_bytes) }}
    </div>
    <button
      class="w-full mt-1 px-2 py-1 rounded bg-emerald-600 hover:bg-emerald-500 disabled:opacity-50 text-xs"
      :disabled="currentProjectId === null"
      @click="$emit('export')"
    >
      导出
    </button>
  </div>
</template>
