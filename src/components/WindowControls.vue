<script setup lang="ts">
import { ref, onMounted } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";

const appWindow = getCurrentWindow();
const maximized = ref(false);

onMounted(async () => {
  maximized.value = await appWindow.isMaximized();
  // 监听最大化状态变化
  await appWindow.onResized(async () => {
    maximized.value = await appWindow.isMaximized();
  });
});

async function minimize() {
  await appWindow.minimize();
}

async function toggleMaximize() {
  await appWindow.toggleMaximize();
}

async function close() {
  await appWindow.close();
}
</script>

<template>
  <div class="flex items-center shrink-0">
    <button
      class="w-11 h-8 flex items-center justify-center text-slate-300 hover:bg-slate-700 transition-colors"
      title="最小化"
      @click="minimize"
    >
      <svg width="12" height="12" viewBox="0 0 12 12">
        <rect x="1" y="5.5" width="10" height="1" fill="currentColor" />
      </svg>
    </button>
    <button
      class="w-11 h-8 flex items-center justify-center text-slate-300 hover:bg-slate-700 transition-colors"
      :title="maximized ? '还原' : '最大化'"
      @click="toggleMaximize"
    >
      <svg v-if="!maximized" width="12" height="12" viewBox="0 0 12 12">
        <rect x="1.5" y="1.5" width="9" height="9" stroke="currentColor" stroke-width="1" fill="none" />
      </svg>
      <svg v-else width="12" height="12" viewBox="0 0 12 12">
        <rect x="1.5" y="3.5" width="7" height="7" stroke="currentColor" stroke-width="1" fill="none" />
        <path d="M3.5 3.5 V1.5 H10.5 V8.5 H8.5" stroke="currentColor" stroke-width="1" fill="none" />
      </svg>
    </button>
    <button
      class="w-11 h-8 flex items-center justify-center text-slate-300 hover:bg-red-600 transition-colors"
      title="关闭"
      @click="close"
    >
      <svg width="12" height="12" viewBox="0 0 12 12">
        <path d="M1 1 L11 11 M11 1 L1 11" stroke="currentColor" stroke-width="1.2" />
      </svg>
    </button>
  </div>
</template>
