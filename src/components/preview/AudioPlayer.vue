<script setup lang="ts">
import { useI18n } from "vue-i18n";
import { getFileUrl } from "../../ipc/fileUrl";
import type { FileNode } from "../../types/library";

const { t } = useI18n();
const props = defineProps<{ file: FileNode }>();
</script>

<template>
  <div class="w-full flex flex-col items-center gap-2 py-4">
    <div class="text-5xl">🎵</div>
    <div class="text-xs text-slate-400 truncate max-w-full">
      {{ props.file.name }}
    </div>
    <audio controls :src="getFileUrl(props.file)" class="w-full max-w-xs">
      {{ t("preview.audioNotSupported") }}
    </audio>
  </div>
</template>
