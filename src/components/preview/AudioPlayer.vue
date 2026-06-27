<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import { getFileUrl } from "../../ipc/fileUrl";
import type { FileNode } from "../../types/library";

const { t } = useI18n();
const props = defineProps<{ file: FileNode }>();

// 用 computed 跟随文件切换，确保切换时音频源更新
const url = computed(() => getFileUrl(props.file));
</script>

<template>
  <div class="w-full flex flex-col items-center gap-2 py-4">
    <div class="text-5xl">🎵</div>
    <div class="text-xs text-slate-400 truncate max-w-full" :title="props.file.name">
      {{ props.file.name }}
    </div>
    <!-- key 绑定 file.id，切换时强制重建 audio 元素，避免播放状态残留 -->
    <audio :key="props.file.id" controls :src="url" class="w-full max-w-xs">
      {{ t("preview.audioNotSupported") }}
    </audio>
  </div>
</template>
