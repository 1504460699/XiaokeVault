<script setup lang="ts">
import { computed } from "vue";
import { storeToRefs } from "pinia";
import { useLibraryStore } from "../stores/libraryStore";
import { useSelectionStore } from "../stores/selectionStore";
import { viewerForKind } from "../utils/viewer";
import SelectionBar from "./SelectionBar.vue";
import ImageViewer from "./preview/ImageViewer.vue";
import ModelViewer from "./preview/ModelViewer.vue";
import TextPreview from "./preview/TextPreview.vue";
import AudioPlayer from "./preview/AudioPlayer.vue";
import FontPreview from "./preview/FontPreview.vue";
import SourcePlaceholder from "./preview/SourcePlaceholder.vue";

const lib = useLibraryStore();
const sel = useSelectionStore();
const { files } = storeToRefs(lib);
const { previewFileId } = storeToRefs(sel);

defineEmits<{ export: [] }>();

const file = computed(
  () => files.value.find((f) => f.id === previewFileId.value) ?? null,
);

const viewer = computed(() =>
  file.value ? viewerForKind(file.value.kind) : "fallback",
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
    <SelectionBar @export="$emit('export')" />
    <div class="px-3 py-2 text-sm text-slate-300 border-b border-slate-700">
      预览
    </div>
    <div v-if="file" class="flex-1 flex flex-col overflow-hidden">
      <!-- 图片/动画/矢量：可缩放的 ImageViewer -->
      <ImageViewer
        v-if="viewer === 'image' || viewer === 'animated' || viewer === 'vector'"
        :file="file"
        class="flex-1 min-h-0"
      />
      <!-- 其他类型：顶对齐，可滚动 -->
      <div
        v-else-if="viewer !== '3d'"
        class="flex-1 overflow-auto bg-slate-900 p-2"
      >
        <AudioPlayer v-if="viewer === 'audio'" :file="file" />
        <FontPreview v-else-if="viewer === 'font'" :file="file" />
        <TextPreview v-else-if="viewer === 'text'" :file="file" />
        <SourcePlaceholder v-else-if="viewer === 'binary-source'" :file="file" />
        <div v-else class="text-5xl text-center py-8">📦</div>
      </div>
      <!-- 3D 模型：独立视口 -->
      <ModelViewer v-else :file="file" class="flex-1 min-h-0" />
      <div class="p-3 text-sm space-y-1 border-t border-slate-700 shrink-0">
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
