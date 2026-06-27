<script setup lang="ts">
import { ref, watch, onUnmounted } from "vue";
import { useI18n } from "vue-i18n";
import { getFileUrl } from "../../ipc/fileUrl";
import type { FileNode } from "../../types/library";

const { t } = useI18n();
const props = defineProps<{ file: FileNode }>();

const waveCanvas = ref<HTMLCanvasElement | null>(null);
const audioEl = ref<HTMLAudioElement | null>(null);
const status = ref(t("preview.loading"));
const duration = ref(0);
const current = ref(0);

let peaks: number[] = []; // 归一化峰值数组
let audioCtx: AudioContext | null = null;
const BARS = 200;

async function loadAudio(f: FileNode) {
  status.value = t("preview.loading");
  peaks = [];
  duration.value = 0;
  current.value = 0;
  try {
    const url = getFileUrl(f);
    const res = await fetch(url);
    const buf = await res.arrayBuffer();
    if (!audioCtx) audioCtx = new AudioContext();
    const audioBuf = await audioCtx.decodeAudioData(buf.slice(0));
    duration.value = audioBuf.duration;
    // 提取第一声道，下采样成 BARS 个峰值
    const data = audioBuf.getChannelData(0);
    const block = Math.floor(data.length / BARS) || 1;
    for (let i = 0; i < BARS; i++) {
      let max = 0;
      const start = i * block;
      for (let j = 0; j < block; j++) {
        const v = Math.abs(data[start + j] || 0);
        if (v > max) max = v;
      }
      peaks.push(max);
    }
    status.value = "";
    drawWave(0);
  } catch (e) {
    status.value = t("preview.loadFailed", { msg: String(e) });
  }
}

// 画波形：progress (0~1) 之前的柱子用高亮色
function drawWave(progress: number) {
  const canvas = waveCanvas.value;
  if (!canvas || peaks.length === 0) return;
  const ctx = canvas.getContext("2d")!;
  const w = canvas.width;
  const h = canvas.height;
  ctx.clearRect(0, 0, w, h);
  const barW = w / peaks.length;
  const mid = h / 2;
  const playedIdx = Math.floor(progress * peaks.length);
  for (let i = 0; i < peaks.length; i++) {
    const barH = Math.max(2, peaks[i] * (h * 0.9));
    ctx.fillStyle = i < playedIdx ? "#38bdf8" : "#475569";
    ctx.fillRect(i * barW + 1, mid - barH / 2, Math.max(1, barW - 1), barH);
  }
}

function onTimeUpdate() {
  if (!audioEl.value || !duration.value) return;
  current.value = audioEl.value.currentTime;
  drawWave(current.value / duration.value);
}

function seek(e: MouseEvent) {
  if (!audioEl.value || !duration.value || peaks.length === 0) return;
  const canvas = waveCanvas.value!;
  const rect = canvas.getBoundingClientRect();
  const ratio = (e.clientX - rect.left) / rect.width;
  audioEl.value.currentTime = ratio * duration.value;
}

function fmtTime(s: number): string {
  const m = Math.floor(s / 60);
  const sec = Math.floor(s % 60);
  return `${m}:${sec.toString().padStart(2, "0")}`;
}

watch(() => props.file, (f) => loadAudio(f), { immediate: true });
onUnmounted(() => {
  audioCtx?.close();
});
</script>

<template>
  <div class="w-full flex flex-col items-center gap-3 py-4">
    <div class="text-5xl">🎵</div>
    <div class="text-xs text-slate-400 truncate max-w-full" :title="props.file.name">
      {{ props.file.name }}
    </div>
    <div v-if="status" class="text-xs text-slate-500">{{ status }}</div>
    <!-- 波形图 -->
    <canvas
      v-show="!status"
      ref="waveCanvas"
      width="600"
      height="80"
      class="w-full max-w-md h-20 cursor-pointer bg-slate-900 rounded"
      @click="seek"
    />
    <!-- 进度时间 -->
    <div v-if="!status" class="text-xs text-slate-400 font-mono">
      {{ fmtTime(current) }} / {{ fmtTime(duration) }}
    </div>
    <!-- 播放器（原生控件）-->
    <audio
      v-if="!status"
      ref="audioEl"
      controls
      :src="getFileUrl(props.file)"
      class="w-full max-w-xs"
      @timeupdate="onTimeUpdate"
    >
      {{ t("preview.audioNotSupported") }}
    </audio>
  </div>
</template>
