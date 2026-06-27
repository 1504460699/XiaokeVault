<script setup lang="ts">
import { computed, ref } from "vue";
import { storeToRefs } from "pinia";
import { useVirtualizer } from "@tanstack/vue-virtual";
import { useLibraryStore } from "../stores/libraryStore";
import { useSelectionStore } from "../stores/selectionStore";
import { getFileUrl } from "../ipc/fileUrl";
import type { FileNode } from "../types/library";

const store = useLibraryStore();
const sel = useSelectionStore();
const { files, currentPackage } = storeToRefs(store);
const { currentProjectId, pkgStates, selectedFileIds } = storeToRefs(sel);

const COLS = 6;
const ROW_H = 150;
const GAP = 12;

// 当前包是否整包勾选
const pkgAllSelected = computed(() => {
  if (store.currentPkgId === null) return false;
  return pkgStates.value[store.currentPkgId] === "all";
});

// 把文件切成行（每行 COLS 个）
const rows = computed<FileNode[][]>(() => {
  const n = files.value.length;
  const r: FileNode[][] = [];
  for (let i = 0; i < n; i += COLS) r.push(files.value.slice(i, i + COLS));
  return r;
});

const parentRef = ref<HTMLElement | null>(null);

const virtualizer = useVirtualizer(
  computed(() => ({
    count: Math.ceil(files.value.length / COLS),
    getScrollElement: () => parentRef.value,
    estimateSize: () => ROW_H + GAP,
    overscan: 4,
  })),
);

const virtualItems = computed(() => virtualizer.value.getVirtualItems());
const totalSize = computed(() => virtualizer.value.getTotalSize());

async function onToggleFile(e: Event, f: FileNode) {
  e.stopPropagation();
  if (currentProjectId.value === null) {
    alert("请先点击右上角“导出”创建项目");
    return;
  }
  if (pkgAllSelected.value) {
    alert("该包已整包勾选。如需精确控制，请先取消整包勾选。");
    return;
  }
  const isSel = selectedFileIds.value.has(f.id);
  await sel.toggleFile(f.id, isSel);
}

function isImage(f: FileNode): boolean {
  return f.kind === "image";
}
</script>

<template>
  <div class="flex-1 flex flex-col overflow-hidden">
    <div
      class="px-4 py-2 text-sm text-slate-400 border-b border-slate-700 shrink-0 flex items-center gap-2"
    >
      <button class="text-sky-400 hover:underline" @click="store.backToPackages()">
        ← 返回包列表
      </button>
      <span>/ {{ currentPackage?.name }}</span>
      <span class="ml-auto">{{ files.length }} 文件</span>
    </div>
    <div ref="parentRef" class="flex-1 overflow-auto p-3">
      <div
        :style="{
          height: `${totalSize}px`,
          position: 'relative',
        }"
      >
        <div
          v-for="vRow in virtualItems"
          :key="vRow.index"
          :style="{
            position: 'absolute',
            top: 0,
            left: 0,
            transform: `translateY(${vRow.start}px)`,
            width: '100%',
            height: `${ROW_H}px`,
            display: 'grid',
            gridTemplateColumns: `repeat(${COLS}, 1fr)`,
            gap: `${GAP}px`,
          }"
        >
          <div
            v-for="f in rows[vRow.index]"
            :key="f.id"
            class="relative bg-slate-800 rounded border border-slate-700 flex flex-col overflow-hidden cursor-pointer hover:border-sky-500"
            :class="
              f.id === sel.previewFileId ? 'border-sky-400 ring-1 ring-sky-400' : ''
            "
            @click="sel.setPreview(f.id)"
          >
            <input
              type="checkbox"
              class="absolute top-1 left-1 z-10 accent-sky-500"
              :checked="pkgAllSelected || selectedFileIds.has(f.id)"
              @click="onToggleFile($event, f)"
            />
            <div
              class="flex-1 flex items-center justify-center bg-slate-900 overflow-hidden"
            >
              <img
                v-if="isImage(f)"
                :src="getFileUrl(f)"
                class="max-w-full max-h-full object-contain"
                loading="lazy"
              />
              <div v-else class="text-3xl">📦</div>
            </div>
            <div class="text-xs text-slate-400 truncate px-1 py-0.5">
              {{ f.name }}
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
