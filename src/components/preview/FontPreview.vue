<script setup lang="ts">
import { ref, watch, onUnmounted } from "vue";
import { getFileUrl } from "../../ipc/fileUrl";
import type { FileNode } from "../../types/library";

const props = defineProps<{ file: FileNode }>();
const fontFamily = ref("");
let styleEl: HTMLStyleElement | null = null;

const SAMPLE =
  "ABCDEFGHIJKLMNOPQRSTUVWXYZ abcdefghijklmnopqrstuvwxyz 0123456789";
const SAMPLE_CN = "游戏素材 字体预览 永和九年岁在癸丑";

watch(
  () => props.file,
  (f) => {
    if (styleEl) {
      document.head.removeChild(styleEl);
      styleEl = null;
    }
    const fam = `preview-font-${f.id}`;
    fontFamily.value = fam;
    const url = getFileUrl(f);
    styleEl = document.createElement("style");
    styleEl.textContent = `@font-face { font-family: "${fam}"; src: url("${url}"); }`;
    document.head.appendChild(styleEl);
  },
  { immediate: true },
);

onUnmounted(() => {
  if (styleEl) document.head.removeChild(styleEl);
});
</script>

<template>
  <div class="w-full flex flex-col gap-3 py-2">
    <div class="text-5xl text-center" :style="{ fontFamily: fontFamily }">
      Aa
    </div>
    <div class="text-sm text-slate-200 px-2" :style="{ fontFamily: fontFamily }">
      {{ SAMPLE }}
    </div>
    <div
      class="text-base text-slate-200 px-2"
      :style="{ fontFamily: fontFamily }"
    >
      {{ SAMPLE_CN }}
    </div>
    <div class="text-xs text-slate-500 px-2">⚠ 字体可能不支持中文</div>
  </div>
</template>
