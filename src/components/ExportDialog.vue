<script setup lang="ts">
import { ref } from "vue";
import { open } from "@tauri-apps/plugin-dialog";
import { storeToRefs } from "pinia";
import { useSelectionStore } from "../stores/selectionStore";
import { exportIpc } from "../ipc/export";
import { listen } from "@tauri-apps/api/event";
import type { ExportProgress } from "../types/export";

const props = defineProps<{ show: boolean }>();
const emit = defineEmits<{ close: [] }>();

const sel = useSelectionStore();
const { currentProjectId } = storeToRefs(sel);

const exportRoot = ref("");
const format = ref<"folder" | "zip">("folder");
const exporting = ref(false);
const progress = ref<ExportProgress | null>(null);
const result = ref<string | null>(null);

async function pickDir() {
  const d = await open({ directory: true, title: "选择导出位置" });
  if (d && !Array.isArray(d)) exportRoot.value = d;
}

async function doExport() {
  if (currentProjectId.value === null) return;
  if (!exportRoot.value) {
    alert("请选择导出位置");
    return;
  }
  exporting.value = true;
  result.value = null;
  progress.value = { stage: "copy", done: 0, total: 0, current: "" };
  const unlisten = await listen<ExportProgress>("export://progress", (e) => {
    progress.value = e.payload;
  });
  try {
    const r = await exportIpc.runExport(
      currentProjectId.value,
      format.value,
      exportRoot.value,
    );
    result.value = r.output_path;
  } catch (e) {
    alert("导出失败：" + String(e));
  } finally {
    exporting.value = false;
    unlisten();
  }
}
</script>

<template>
  <Teleport to="body">
    <div
      v-if="props.show"
      class="fixed inset-0 bg-black/60 flex items-center justify-center z-50"
      @click.self="emit('close')"
    >
      <div class="bg-slate-800 rounded-lg p-6 w-[480px] text-slate-100 space-y-3">
        <h2 class="text-lg font-bold">导出项目</h2>

        <div v-if="!exporting && !result">
          <label class="block text-sm text-slate-400 mb-1">导出位置</label>
          <div class="flex gap-2 mb-3">
            <input
              :value="exportRoot"
              readonly
              class="flex-1 bg-slate-700 rounded px-2 py-1 text-sm"
              placeholder="选择文件夹"
            />
            <button
              class="px-3 py-1 rounded bg-slate-600 hover:bg-slate-500 text-sm"
              @click="pickDir"
            >
              浏览
            </button>
          </div>

          <label class="block text-sm text-slate-400 mb-1">格式</label>
          <div class="flex gap-4 mb-4">
            <label class="flex items-center gap-1 text-sm">
              <input type="radio" value="folder" v-model="format" /> 文件夹
            </label>
            <label class="flex items-center gap-1 text-sm">
              <input type="radio" value="zip" v-model="format" /> zip 压缩包
            </label>
          </div>

          <div class="flex justify-end gap-2">
            <button
              class="px-4 py-1 rounded bg-slate-600 hover:bg-slate-500 text-sm"
              @click="emit('close')"
            >
              取消
            </button>
            <button
              class="px-4 py-1 rounded bg-sky-600 hover:bg-sky-500 text-sm"
              @click="doExport"
            >
              开始导出
            </button>
          </div>
        </div>

        <div v-else-if="exporting" class="py-4">
          <div class="text-sm mb-2">
            {{ progress?.stage === "copy" ? "复制中" : "处理中" }}…
            ({{ progress?.done }}/{{ progress?.total }})
          </div>
          <div class="w-full bg-slate-700 rounded h-2 overflow-hidden">
            <div
              class="bg-sky-500 h-full transition-all"
              :style="{
                width:
                  progress && progress.total
                    ? (progress.done / progress.total) * 100 + '%'
                    : '0%',
              }"
            ></div>
          </div>
          <div class="text-xs text-slate-400 mt-1 truncate">{{ progress?.current }}</div>
        </div>

        <div v-else class="py-4">
          <div class="text-emerald-400 text-sm mb-2">✓ 导出完成</div>
          <div class="text-xs text-slate-400 break-all mb-3">{{ result }}</div>
          <div class="flex justify-end">
            <button
              class="px-4 py-1 rounded bg-slate-600 hover:bg-slate-500 text-sm"
              @click="emit('close')"
            >
              关闭
            </button>
          </div>
        </div>
      </div>
    </div>
  </Teleport>
</template>
