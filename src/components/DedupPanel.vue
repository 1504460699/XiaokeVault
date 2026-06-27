<script setup lang="ts">
import { ref } from "vue";
import { storeToRefs } from "pinia";
import { open } from "@tauri-apps/plugin-dialog";
import { useDedupStore } from "../stores/dedupStore";
import { useLibraryStore } from "../stores/libraryStore";

const props = defineProps<{ show: boolean }>();
const emit = defineEmits<{ close: [] }>();

const dedup = useDedupStore();
const lib = useLibraryStore();const { groups, report, scanning, removing } = storeToRefs(dedup);

const backupRoot = ref("");
const lastResult = ref<string | null>(null);

async function onScan() {
  if (lib.currentLibId === null) return;
  lastResult.value = null;
  await dedup.runDedup(lib.currentLibId);
}

async function pickBackupDir() {
  const d = await open({ directory: true, title: "选择备份位置（删除的文件会移到这里）" });
  if (d && !Array.isArray(d)) backupRoot.value = d;
}

async function onRemove(groupId: number, fileId: number | null) {
  if (fileId === null) return;
  if (!backupRoot.value) {
    alert("请先选择备份位置");
    return;
  }
  if (!confirm("确认删除？文件会移到备份目录，可恢复。")) return;
  await dedup.removeMember(fileId, groupId, backupRoot.value);
}

async function onRemoveAll() {
  if (groups.value.length === 0) return;
  if (!backupRoot.value) {
    alert("请先选择备份位置");
    return;
  }
  if (!confirm(`确认一键清理 ${report.value?.removable_files ?? 0} 个冗余文件？\n文件会移到：${backupRoot.value}`)) return;
  const r = await dedup.removeAll(backupRoot.value);
  lastResult.value = `已清理 ${r.removed} 个文件${r.failed ? `，${r.failed} 个失败` : ""}`;
}

// 跳转到包（关闭面板，定位到该包）
async function locatePackage(pkgId: number | null) {
  if (pkgId === null) return;
  emit("close");
  await lib.selectPackage(pkgId);
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
        class="bg-slate-800 rounded-lg p-6 w-[640px] max-h-[85vh] flex flex-col text-slate-100"
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

        <!-- 备份位置 -->
        <div class="mb-3">
          <label class="block text-xs text-slate-400 mb-1">删除备份位置（删掉的文件移到这里，可恢复）</label>
          <div class="flex gap-2">
            <input
              :value="backupRoot"
              readonly
              placeholder="未选择则用应用默认 trash 目录"
              class="flex-1 bg-slate-700 rounded px-2 py-1 text-xs"
            />
            <button
              class="px-3 py-1 rounded bg-slate-600 hover:bg-slate-500 text-xs"
              @click="pickBackupDir"
            >
              选择
            </button>
          </div>
        </div>

        <div v-if="report" class="text-xs text-slate-400 mb-2">
          发现 {{ report.groups }} 组重复 · 可清理
          {{ report.removable_files }} 文件 ·
          {{ fmtBytes(report.removable_bytes) }}
          <button
            v-if="groups.length > 0"
            class="ml-3 px-2 py-0.5 rounded bg-red-700 hover:bg-red-600 text-xs"
            :disabled="removing"
            @click="onRemoveAll"
          >
            {{ removing ? "清理中…" : "一键清理全部" }}
          </button>
        </div>

        <div v-if="lastResult" class="text-xs text-emerald-400 mb-2">{{ lastResult }}</div>

        <div class="flex-1 overflow-auto space-y-2">
          <div
            v-for="g in groups"
            :key="g.id"
            class="bg-slate-900 rounded p-3 border border-slate-700"
          >
            <div
              class="text-sm mb-1"
              :class="g.reason === 'likely_backup' ? 'text-orange-400' : 'text-amber-400'"
            >
              ⚠ {{ g.reason === 'likely_backup' ? '疑似备份（需人工确认）' : '冗余文件' }}
            </div>
            <div class="text-xs text-slate-300 mb-2">{{ g.detail }}</div>
            <!-- likely_backup：人工处理按钮 -->
            <div v-if="g.reason === 'likely_backup'" class="flex gap-2 mb-2">
              <button
                class="px-2 py-0.5 rounded bg-slate-600 hover:bg-slate-500 text-xs whitespace-nowrap"
                @click="dedup.dismissGroup(g.id)"
              >
                忽略（不是重复）
              </button>
              <button
                class="px-2 py-0.5 rounded bg-emerald-700 hover:bg-emerald-600 text-xs whitespace-nowrap"
                @click="dedup.dismissGroup(g.id)"
              >
                确认是备份（保留，不再提醒）
              </button>
            </div>
            <div
              v-for="m in g.members"
              :key="m.file_id ?? m.package_id ?? 0"
              class="text-xs text-slate-400 flex items-center gap-2"
            >
              <span class="flex-1 truncate">
                {{ m.package_name }}{{ m.rel_path ? ' / ' + m.rel_path : '' }}
              </span>
              <button
                v-if="m.role === 'remove' && m.file_id"
                class="px-2 py-0.5 rounded bg-red-700 hover:bg-red-600 text-xs whitespace-nowrap"
                @click="onRemove(g.id, m.file_id)"
              >
                删除
              </button>
              <span v-else-if="g.reason === 'likely_backup'" class="flex items-center gap-1">
                <span class="text-slate-500">需人工判断</span>
                <button
                  v-if="m.package_id"
                  class="px-1.5 py-0.5 rounded bg-slate-600 hover:bg-sky-600 text-slate-300 text-xs whitespace-nowrap"
                  @click="locatePackage(m.package_id)"
                >
                  打开包
                </button>
              </span>
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
