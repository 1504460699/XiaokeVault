<script setup lang="ts">
import { ref, watch, onUnmounted } from "vue";
import { useI18n } from "vue-i18n";
import { parseGIF, decompressFrames } from "gifuct-js";
import type { FileNode } from "../../types/library";
import { getFileUrl } from "../../ipc/fileUrl";

const { t } = useI18n();
const props = defineProps<{ file: FileNode }>();

interface Frame {
  dims: { width: number; height: number; top: number; left: number };
  patch: Uint8ClampedArray; // RGBA
  delay: number; // ms
  disposalType: number;
}

const canvasRef = ref<HTMLCanvasElement | null>(null);
const status = ref(t("preview.loading"));
const playing = ref(true);
const speed = ref(1);
const currentFrame = ref(0);
const frameCount = ref(0);

let frames: Frame[] = [];
let rafId = 0;
let lastTime = 0;
let accDelay = 0;

async function loadGif(f: FileNode) {
  status.value = t("preview.loading");
  playing.value = true;
  currentFrame.value = 0;
  try {
    const url = getFileUrl(f);
    const res = await fetch(url);
    const buf = await res.arrayBuffer();
    const gif = parseGIF(buf);
    const parsed = decompressFrames(gif, true);
    frames = parsed as unknown as Frame[];
    frameCount.value = frames.length;
    if (frames.length === 0) {
      status.value = t("preview.gifEmpty");
      return;
    }
    status.value = "";
    const canvas = canvasRef.value!;
    // 用首帧尺寸作为画布尺寸
    canvas.width = frames[0].dims.width + frames[0].dims.left;
    canvas.height = frames[0].dims.height + frames[0].dims.top;
    drawFrame(0);
    startLoop();
  } catch (e) {
    status.value = t("preview.loadFailed", { msg: String(e) });
  }
}

// 画第 i 帧（含处理 disposal：简单实现，逐帧全量画 patch）
function drawFrame(i: number) {
  const canvas = canvasRef.value;
  if (!canvas) return;
  const ctx = canvas.getContext("2d")!;
  const fr = frames[i];
  if (!fr) return;
  const imgData = new ImageData(
    new Uint8ClampedArray(fr.patch),
    fr.dims.width,
    fr.dims.height,
  );
  ctx.putImageData(imgData, fr.dims.left, fr.dims.top);
}

function startLoop() {
  cancelAnimationFrame(rafId);
  lastTime = performance.now();
  accDelay = 0;
  const loop = (now: number) => {
    rafId = requestAnimationFrame(loop);
    if (!playing.value || frames.length === 0) return;
    const dt = now - lastTime;
    lastTime = now;
    accDelay += dt * speed.value;
    const delay = frames[currentFrame.value]?.delay || 100;
    if (accDelay >= delay) {
      accDelay = 0;
      currentFrame.value = (currentFrame.value + 1) % frames.length;
      drawFrame(currentFrame.value);
    }
  };
  rafId = requestAnimationFrame(loop);
}

function togglePlay() {
  playing.value = !playing.value;
  if (playing.value) {
    lastTime = performance.now();
    accDelay = 0;
  }
}

function seek(idx: number) {
  currentFrame.value = (idx + frames.length) % frames.length;
  drawFrame(currentFrame.value);
  accDelay = 0;
}

function step(dir: number) {
  playing.value = false;
  seek(currentFrame.value + dir);
}

watch(() => props.file, (f) => loadGif(f), { immediate: true });
onUnmounted(() => cancelAnimationFrame(rafId));
</script>

<template>
  <div class="w-full h-full flex flex-col">
    <div class="flex-1 min-h-0 flex items-center justify-center bg-slate-900 overflow-hidden relative">
      <canvas v-show="!status" ref="canvasRef" class="max-w-full max-h-full object-contain" />
      <div v-if="status" class="text-xs text-slate-400">{{ status }}</div>
    </div>
    <!-- 控制台：固定行高，统一按钮/下拉框高度，进度条可压缩防溢出 -->
    <div
      v-if="!status && frameCount > 0"
      class="flex items-center gap-2 px-3 py-2 bg-slate-800 border-t border-slate-700 text-xs shrink-0"
    >
      <button
        type="button"
        class="h-6 w-7 shrink-0 rounded bg-slate-600 hover:bg-slate-500 leading-none"
        @click="step(-1)"
        :title="t('preview.prevFrame')"
      >◀|</button>
      <button
        type="button"
        class="h-6 w-8 shrink-0 rounded bg-sky-600 hover:bg-sky-500 leading-none"
        @click="togglePlay"
      >{{ playing ? "⏸" : "▶" }}</button>
      <button
        type="button"
        class="h-6 w-7 shrink-0 rounded bg-slate-600 hover:bg-slate-500 leading-none"
        @click="step(1)"
        :title="t('preview.nextFrame')"
      >|▶</button>
      <span class="shrink-0 text-slate-400 whitespace-nowrap tabular-nums">{{ currentFrame + 1 }}/{{ frameCount }}</span>
      <!-- 进度条：可压缩，min-w-0 防止撑爆容器 -->
      <input
        type="range"
        :min="0"
        :max="frameCount - 1"
        :value="currentFrame"
        class="flex-1 min-w-0 accent-sky-500"
        @input="seek(Number(($event.target as HTMLInputElement).value))"
      />
      <!-- 调速：固定高度与按钮一致，shrink-0 防被压缩 -->
      <select
        v-model="speed"
        class="h-6 w-14 shrink-0 rounded bg-slate-700 text-slate-100 leading-none text-center"
        :title="t('preview.speed')"
      >
        <option :value="0.25">0.25×</option>
        <option :value="0.5">0.5×</option>
        <option :value="1">1×</option>
        <option :value="2">2×</option>
        <option :value="4">4×</option>
      </select>
    </div>
  </div>
</template>
