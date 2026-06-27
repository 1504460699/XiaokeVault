<script setup lang="ts">
import type { FileNode } from "../../types/library";

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
    <div class="text-sm text-slate-300">源文件</div>
    <div class="text-xs text-slate-500 text-center px-4">
      {{ SOFTWARE[props.file.ext] ?? "专业" }} 格式，需用对应软件打开编辑
    </div>
  </div>
</template>
