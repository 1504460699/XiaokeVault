<script setup lang="ts">
import { storeToRefs } from "pinia";
import { useSelectionStore } from "../stores/selectionStore";

const sel = useSelectionStore();
const { summary, currentProjectId, projects } = storeToRefs(sel);

defineEmits<{ export: [] }>();

async function onClear() {
  if (summary.value.file_count === 0) return;
  if (!confirm(`清空所有 ${summary.value.file_count} 个文件的勾选？`)) return;
  await sel.clearAll();
}

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
      class="w-full mt-1 px-2 py-1 rounded bg-emerald-600 hover:bg-emerald-500 text-xs"
      @click="$emit('export')"
    >
      {{ currentProjectId === null ? "创建项目并导出" : "导出" }}
    </button>
    <button
      v-if="summary.file_count > 0"
      class="w-full px-2 py-0.5 rounded bg-slate-600 hover:bg-slate-500 text-xs text-slate-300"
      @click="onClear"
    >
      清空选择
    </button>
  </div>
</template>
