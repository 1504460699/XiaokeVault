<script setup lang="ts">
import { getFileUrl } from "../../ipc/fileUrl";
import type { FileNode } from "../../types/library";

const props = defineProps<{ file: FileNode }>();
</script>

<template>
  <div class="w-full flex flex-col items-center gap-2 py-4">
    <div class="text-5xl">🎵</div>
    <div class="text-xs text-slate-400 truncate max-w-full">
      {{ props.file.name }}
    </div>
    <audio controls :src="getFileUrl(props.file)" class="w-full max-w-xs">
      浏览器不支持音频播放
    </audio>
  </div>
</template>
