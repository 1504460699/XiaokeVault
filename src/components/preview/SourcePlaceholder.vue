<script setup lang="ts">
import { useI18n } from "vue-i18n";
import type { FileNode } from "../../types/library";

const { t } = useI18n();
const props = defineProps<{ file: FileNode }>();

const SOFTWARE: Record<string, string> = {
  blend: "Blender",
  psd: "Photoshop",
  xcf: "GIMP",
  ase: "Aseprite",
  ai: "Illustrator",
};
</script>

<template>
  <div class="w-full flex flex-col items-center gap-2 py-6">
    <div class="text-5xl">⚙️</div>
    <div class="text-sm text-slate-300">{{ t("preview.sourceFile") }}</div>
    <div class="text-xs text-slate-500 text-center px-4">
      {{ t("preview.sourceFormat", { software: SOFTWARE[props.file.ext] ?? t("preview.professional") }) }}
    </div>
  </div>
</template>
