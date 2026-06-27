<script setup lang="ts">
import { ref, computed, watch } from "vue";
import { useI18n } from "vue-i18n";
import { getFileUrl } from "../../ipc/fileUrl";
import type { FileNode } from "../../types/library";

const { t } = useI18n();
const props = defineProps<{ file: FileNode }>();

const scale = ref(1);
const x = ref(0);
const y = ref(0);
const dragging = ref(false);
let lastX = 0;
let lastY = 0;

// url 用 computed 跟随 props.file 变化，切换文件时立即更新
const url = computed(() => getFileUrl(props.file));

// 切换文件时重置缩放/平移状态
watch(
  () => props.file.id,
  () => {
    scale.value = 1;
    x.value = 0;
    y.value = 0;
  },
);

function onWheel(e: WheelEvent) {
  e.preventDefault();
  const delta = e.deltaY > 0 ? 0.9 : 1.1;
  scale.value = Math.min(8, Math.max(0.1, scale.value * delta));
}

function onDown(e: MouseEvent) {
  dragging.value = true;
  lastX = e.clientX;
  lastY = e.clientY;
}

function onMove(e: MouseEvent) {
  if (!dragging.value) return;
  x.value += e.clientX - lastX;
  y.value += e.clientY - lastY;
  lastX = e.clientX;
  lastY = e.clientY;
}

function onUp() {
  dragging.value = false;
}

function reset() {
  scale.value = 1;
  x.value = 0;
  y.value = 0;
}

function fit() {
  reset();
}

function zoom(factor: number) {
  scale.value = Math.min(8, Math.max(0.1, scale.value * factor));
}
</script>

<template>
  <div class="w-full h-full flex flex-col">
    <!-- 缩放工具栏 -->
    <div
      class="flex items-center gap-2 px-2 py-1 bg-slate-800 border-b border-slate-700 text-xs shrink-0"
    >
      <button class="px-2 py-0.5 rounded bg-slate-600 hover:bg-slate-500" @click="zoom(0.8)">
        −
      </button>
      <span class="text-slate-300 w-12 text-center">{{ Math.round(scale * 100) }}%</span>
      <button class="px-2 py-0.5 rounded bg-slate-600 hover:bg-slate-500" @click="zoom(1.25)">
        +
      </button>
      <button
        class="px-2 py-0.5 rounded bg-slate-600 hover:bg-slate-500 flex items-center justify-center"
        :title="t('preview.fit')"
        @click="fit"
      >
        <!-- 适应/contain：方框内四角向内的箭头 -->
        <svg width="14" height="14" viewBox="0 0 16 16" fill="none">
          <path d="M2 5 V2 H5 M11 2 H14 V5 M14 11 V14 H11 M5 14 H2 V11" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"/>
        </svg>
      </button>
      <button class="px-2 py-0.5 rounded bg-slate-600 hover:bg-slate-500" @click="reset">
        100%
      </button>
      <span class="text-slate-500 ml-auto">{{ t("preview.imageZoomHint") }}</span>
    </div>
    <!-- 画布 -->
    <div
      class="flex-1 overflow-hidden relative bg-slate-900 flex items-center justify-center cursor-grab"
      :class="dragging ? 'cursor-grabbing' : ''"
      @wheel="onWheel"
      @mousedown="onDown"
      @mousemove="onMove"
      @mouseup="onUp"
      @mouseleave="onUp"
    >
      <img
        :src="url"
        :style="{ transform: `translate(${x}px, ${y}px) scale(${scale})`, maxWidth: '100%', maxHeight: '100%', objectFit: 'contain' }"
        draggable="false"
        class="select-none"
        @dblclick="reset"
      />
    </div>
  </div>
</template>
