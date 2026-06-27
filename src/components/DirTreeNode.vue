<script setup lang="ts">
import { ref } from "vue";
import { storeToRefs } from "pinia";
import { useTreeStore } from "../stores/treeStore";
import type { DirNode } from "../types/library";

const props = defineProps<{ node: DirNode }>();
const store = useTreeStore();
const { currentDirId } = storeToRefs(store);

// 默认只展开第一层（depth=0）
const expanded = ref(props.node.depth === 0);

function fmtBytes(b: number): string {
  if (b > 1e9) return (b / 1e9).toFixed(1) + " GB";
  if (b > 1e6) return (b / 1e6).toFixed(1) + " MB";
  if (b > 1e3) return (b / 1e3).toFixed(0) + " KB";
  return b + " B";
}

function onClickName() {
  if (props.node.file_count > 0) {
    store.selectDirectory(props.node.id);
  }
}
</script>

<template>
  <div>
    <div
      class="flex items-center gap-1 px-2 py-1 cursor-pointer rounded hover:bg-slate-700/50 text-sm"
      :class="node.id === currentDirId ? 'bg-sky-600/30' : ''"
      :style="{ paddingLeft: node.depth * 12 + 8 + 'px' }"
    >
      <!-- 展开/折叠箭头（有子节点才显示）-->
      <span
        v-if="node.children.length > 0"
        class="text-xs text-slate-500 w-3 inline-block select-none"
        @click.stop="expanded = !expanded"
      >{{ expanded ? '▼' : '▶' }}</span>
      <span v-else class="inline-block w-3"></span>
      <!-- 文件夹图标 -->
      <span class="text-amber-400">{{ expanded && node.children.length ? '📂' : '📁' }}</span>
      <!-- 名称 -->
      <span
        class="flex-1 truncate"
        :class="node.file_count > 0 ? 'text-slate-200' : 'text-slate-500'"
        :title="node.path"
        @click="onClickName"
      >{{ node.name }}</span>
      <!-- 统计 -->
      <span v-if="node.file_count > 0" class="text-xs text-slate-500 whitespace-nowrap">
        {{ node.file_count }} · {{ fmtBytes(node.total_bytes) }}
      </span>
    </div>
    <!-- 递归子节点 -->
    <div v-if="expanded">
      <DirTreeNode
        v-for="child in node.children"
        :key="child.id"
        :node="child"
      />
    </div>
  </div>
</template>
