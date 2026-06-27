<script setup lang="ts">
import { computed } from "vue";
import { storeToRefs } from "pinia";
import { useLibraryStore } from "../stores/libraryStore";
import { useSelectionStore } from "../stores/selectionStore";
import { getFileUrl } from "../ipc/fileUrl";

const lib = useLibraryStore();
const sel = useSelectionStore();
const { files } = storeToRefs(lib);
const { previewFileId } = storeToRefs(sel);

const file = computed(
  () => files.value.find((f) => f.id === previewFileId.value) ?? null,
);

function fmtBytes(b: number): string {
  if (b > 1e6) return (b / 1e6).toFixed(1) + " MB";
  if (b > 1e3) return (b / 1e3).toFixed(0) + " KB";
  return b + " B";
}
</script>

<template>
  <aside
    class="w-80 shrink-0 border-l border-slate-700 bg-slate-800/50 flex flex-col"
  >
    <div class="px-3 py-2 text-sm text-slate-300 border-b border-slate-700">
      预览
    </div>
    <div v-if="file" class="flex-1 flex flex-col overflow-auto">
      <div class="flex-1 flex items-center justify-center bg-slate-900 min-h-48">
        <img
          v-if="file.kind === 'image'"
          :src="getFileUrl(file)"
          class="max-w-full max-h-96 object-contain"
        />
        <div v-else class="text-5xl">📦</div>
      </div>
      <div class="p-3 text-sm space-y-1">
        <div class="font-medium break-all">{{ file.name }}</div>
        <div class="text-slate-400">
          {{ fmtBytes(file.bytes) }} · {{ file.ext }} · {{ file.kind }}
        </div>
        <div class="text-slate-500 text-xs break-all">{{ file.rel_path }}</div>
      </div>
    </div>
    <div
      v-else
      class="flex-1 flex items-center justify-center text-slate-500 text-sm"
    >
      点击文件预览
    </div>
  </aside>
</template>
