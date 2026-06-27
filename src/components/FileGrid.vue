<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { storeToRefs } from "pinia";
import { useVirtualizer } from "@tanstack/vue-virtual";
import { useLibraryStore } from "../stores/libraryStore";
import { useSelectionStore } from "../stores/selectionStore";
import { useSearchStore } from "../stores/searchStore";
import { getFileUrl } from "../ipc/fileUrl";
import { viewerForKind, iconForViewer, canShowThumb } from "../utils/viewer";
import type { FileNode } from "../types/library";

// 外部请求定位到的文件 id（来自搜索定位）
const props = defineProps<{ locateFileId?: number | null }>();
const emit = defineEmits<{ located: [] }>();

const store = useLibraryStore();
const sel = useSelectionStore();
const search = useSearchStore();
const { files, currentPackage } = storeToRefs(store);
const { pkgStates, selectedFileIds } = storeToRefs(sel);

// 返回：搜索模式下回搜索结果，否则回包列表
function onBack() {
  if (search.active) {
    store.backToPackages(); // currentPkgId=null → SearchView 显示
  } else {
    store.backToPackages();
  }
}

const COLS = 6;
const ROW_H = 150;
const GAP = 12;

// 搜索与过滤状态
const searchQuery = ref("");
const filterKind = ref(""); // 空=全部

// 过滤后的文件（按文件名 + 类型）
const filteredFiles = computed<FileNode[]>(() => {
  const q = searchQuery.value.trim().toLowerCase();
  const k = filterKind.value;
  return files.value.filter((f) => {
    if (k && f.kind !== k) return false;
    if (q && !f.name.toLowerCase().includes(q)) return false;
    return true;
  });
});

// 当前可用的类型（供下拉，从当前包文件动态生成）
const availableKinds = computed(() => {
  const set = new Map<string, number>();
  for (const f of files.value) {
    set.set(f.kind, (set.get(f.kind) ?? 0) + 1);
  }
  return Array.from(set.entries()).map(([kind, count]) => ({ kind, count }));
});

// 当前包是否整包勾选
const pkgAllSelected = computed(() => {
  if (store.currentPkgId === null) return false;
  return pkgStates.value[store.currentPkgId] === "all";
});

// 把文件切成行（每行 COLS 个）——基于过滤后的文件
const rows = computed<FileNode[][]>(() => {
  const n = filteredFiles.value.length;
  const r: FileNode[][] = [];
  for (let i = 0; i < n; i += COLS) r.push(filteredFiles.value.slice(i, i + COLS));
  return r;
});

const parentRef = ref<HTMLElement | null>(null);

const virtualizer = useVirtualizer(
  computed(() => ({
    count: Math.ceil(filteredFiles.value.length / COLS),
    getScrollElement: () => parentRef.value,
    estimateSize: () => ROW_H + GAP,
    overscan: 4,
  })),
);

const virtualItems = computed(() => virtualizer.value.getVirtualItems());
const totalSize = computed(() => virtualizer.value.getTotalSize());

// 定位到指定文件：计算其在过滤后列表的索引，滚动到对应行
watch(
  () => props.locateFileId,
  (fid) => {
    if (fid == null) return;
    // 等 files 渲染后再滚动（nextTick）
    setTimeout(() => {
      const idx = filteredFiles.value.findIndex((f) => f.id === fid);
      if (idx >= 0) {
        const rowIndex = Math.floor(idx / COLS);
        virtualizer.value.scrollToIndex(rowIndex, { align: "center" });
      }
      emit("located");
    }, 100);
  },
);

async function onToggleFile(e: Event, f: FileNode) {
  e.stopPropagation();
  await sel.ensureProject();
  if (pkgAllSelected.value) {
    alert("该包已整包勾选。如需精确控制，请先取消整包勾选。");
    return;
  }
  const isSel = selectedFileIds.value.has(f.id);
  await sel.toggleFile(f.id, isSel);
}
</script>

<template>
  <div class="flex-1 flex flex-col overflow-hidden">
    <div
      class="px-4 py-2 text-sm text-slate-400 border-b border-slate-700 shrink-0 flex items-center gap-2 flex-wrap"
    >
      <button class="text-sky-400 hover:underline" @click="onBack()">
        ← {{ search.active ? '返回搜索结果' : '返回包列表' }}
      </button>
      <span>/ {{ currentPackage?.name }}</span>
      <span class="text-slate-500">
        {{ filteredFiles.length }}/{{ files.length }} 文件
      </span>
      <div class="ml-auto flex items-center gap-2">
        <select
          v-model="filterKind"
          class="bg-slate-700 text-slate-200 px-2 py-0.5 rounded text-xs"
        >
          <option value="">全部类型</option>
          <option v-for="k in availableKinds" :key="k.kind" :value="k.kind">
            {{ k.kind }} ({{ k.count }})
          </option>
        </select>
        <input
          v-model="searchQuery"
          type="text"
          placeholder="搜索文件名…"
          class="bg-slate-700 text-slate-200 px-2 py-0.5 rounded text-xs w-40"
        />
      </div>
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
                v-if="canShowThumb(viewerForKind(f.kind))"
                :src="getFileUrl(f)"
                class="max-w-full max-h-full object-contain"
                loading="lazy"
              />
              <div v-else class="text-3xl">
                {{ iconForViewer(viewerForKind(f.kind)) }}
              </div>
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
