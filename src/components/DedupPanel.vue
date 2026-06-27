<script setup lang="ts">
import { storeToRefs } from "pinia";
import { useDedupStore } from "../stores/dedupStore";
import { useLibraryStore } from "../stores/libraryStore";

const props = defineProps<{ show: boolean }>();
const emit = defineEmits<{ close: [] }>();

const dedup = useDedupStore();
const lib = useLibraryStore();
const { groups, report, scanning } = storeToRefs(dedup);

async function onScan() {
  if (lib.currentLibId === null) return;
  await dedup.runDedup(lib.currentLibId);
}

async function onRemove(groupId: number, fileId: number | null) {
  if (fileId === null) return;
  if (!confirm("确认删除？文件会移到 trash 目录，可恢复。")) return;
  await dedup.removeMember(fileId, groupId);
}

function fmtBytes(b: number): string {
  if (b > 1e9) return (b / 1e9).toFixed(2) + " GB";
  if (b > 1e6) return (b / 1e6).toFixed(1) + " MB";
  if (b > 1e3) return (b / 1e3).toFixed(0) + " KB";
  return b + " B";
}
</script>

<template>
  <Teleport to="body">
    <div
      v-if="props.show"
      class="fixed inset-0 bg-black/60 flex items-center justify-center z-50"
      @click.self="emit('close')"
    >
      <div
        class="bg-slate-800 rounded-lg p-6 w-[640px] max-h-[80vh] flex flex-col text-slate-100"
      >
        <div class="flex items-center mb-3">
          <h2 class="text-lg font-bold flex-1">去重整理</h2>
          <button
            class="px-3 py-1 rounded bg-sky-600 hover:bg-sky-500 text-sm disabled:opacity-50"
            :disabled="scanning || lib.currentLibId === null"
            @click="onScan"
          >
            {{ scanning ? "检测中…" : groups.length ? "重新检测" : "开始检测" }}
          </button>
        </div>

        <div v-if="report" class="text-xs text-slate-400 mb-3">
          发现 {{ report.groups }} 组重复 · 可清理
          {{ report.removable_files }} 文件 ·
          {{ fmtBytes(report.removable_bytes) }}
        </div>

        <div class="flex-1 overflow-auto space-y-2">
          <div
            v-for="g in groups"
            :key="g.id"
            class="bg-slate-900 rounded p-3 border border-slate-700"
          >
            <div class="text-sm text-amber-400 mb-1">⚠ {{ g.reason }}</div>
            <div class="text-xs text-slate-300 mb-2">{{ g.detail }}</div>
            <div
              v-for="m in g.members"
              :key="m.file_id ?? 0"
              class="text-xs text-slate-400 flex items-center gap-2"
            >
              <span class="flex-1 truncate"
                >{{ m.package_name }} / {{ m.rel_path }}</span
              >
              <button
                v-if="m.role === 'remove' && m.file_id"
                class="px-2 py-0.5 rounded bg-red-700 hover:bg-red-600 text-xs"
                @click="onRemove(g.id, m.file_id)"
              >
                删除
              </button>
            </div>
          </div>
          <div
            v-if="groups.length === 0 && report"
            class="text-center text-slate-500 py-8"
          >
            ✓ 未发现重复
          </div>
          <div v-if="!report" class="text-center text-slate-500 py-8">
            点击「开始检测」扫描重复
          </div>
        </div>

        <div class="flex justify-end mt-3">
          <button
            class="px-4 py-1 rounded bg-slate-600 hover:bg-slate-500 text-sm"
            @click="emit('close')"
          >
            关闭
          </button>
        </div>
      </div>
    </div>
  </Teleport>
</template>
