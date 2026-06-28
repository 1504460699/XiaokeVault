<script setup lang="ts">
import { computed, defineAsyncComponent } from "vue";
import { storeToRefs } from "pinia";
import { useI18n } from "vue-i18n";
import { useTreeStore } from "../stores/treeStore";
import { useSelectionStore } from "../stores/selectionStore";
import { viewerForKind } from "../utils/viewer";
import SelectionBar from "./SelectionBar.vue";

const { t } = useI18n();
import ImageViewer from "./preview/ImageViewer.vue";
import GifPlayer from "./preview/GifPlayer.vue";
// ModelViewer 含 three.js，按需懒加载（进 3D 预览才加载）
const ModelViewer = defineAsyncComponent(() => import("./preview/ModelViewer.vue"));
import TextPreview from "./preview/TextPreview.vue";
import AudioPlayer from "./preview/AudioPlayer.vue";
import FontPreview from "./preview/FontPreview.vue";
import SourcePlaceholder from "./preview/SourcePlaceholder.vue";

const tree = useTreeStore();
const sel = useSelectionStore();
const { files: treeFiles } = storeToRefs(tree);
const { previewFileId } = storeToRefs(sel);

defineEmits<{ export: [] }>();

// 当前预览文件：从 treeStore.files 查找
const file = computed(() => {
  const id = previewFileId.value;
  if (id == null) return null;
  return treeFiles.value.find((f) => f.id === id) ?? null;
});

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
      {{ t("preview.title") }}
    </div>
    <div v-if="file" class="flex-1 flex flex-col overflow-hidden">
      <!-- GIF 动画：逐帧播放器 -->
      <GifPlayer v-if="viewer === 'animated'" :file="file" class="flex-1 min-h-0" />
      <!-- 图片/矢量：可缩放的 ImageViewer -->
      <ImageViewer
        v-else-if="viewer === 'image' || viewer === 'vector'"
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
        <div class="font-medium truncate" :title="file.name">{{ file.name }}</div>
        <div class="text-slate-400">
          {{ fmtBytes(file.bytes) }} · {{ file.ext }} · {{ file.kind }}
        </div>
        <div class="text-slate-500 text-xs truncate" :title="file.rel_path">{{ file.rel_path }}</div>
      </div>
    </div>
    <div
      v-else
      class="flex-1 flex items-center justify-center text-slate-500 text-sm"
    >
      {{ t("preview.noSelection") }}
    </div>
  </aside>
</template>
