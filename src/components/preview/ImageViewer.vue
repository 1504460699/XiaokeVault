<script setup lang="ts">
import { ref } from "vue";
import { getFileUrl } from "../../ipc/fileUrl";
import type { FileNode } from "../../types/library";

const props = defineProps<{ file: FileNode }>();

const scale = ref(1);
const x = ref(0);
const y = ref(0);
const dragging = ref(false);
let lastX = 0;
let lastY = 0;

const url = getFileUrl(props.file);

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
      <button class="px-2 py-0.5 rounded bg-slate-600 hover:bg-slate-500" @click="fit">
        适应
      </button>
      <button class="px-2 py-0.5 rounded bg-slate-600 hover:bg-slate-500" @click="reset">
        100%
      </button>
      <span class="text-slate-500 ml-auto">滚轮缩放 · 拖拽移动</span>
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
