<script setup lang="ts">
import { ref } from "vue";
import { storeToRefs } from "pinia";
import { useTreeStore } from "../stores/treeStore";
import { useLibraryStore } from "../stores/libraryStore";
import type { DirNode } from "../types/library";

const props = defineProps<{ node: DirNode; isRoot?: boolean }>();
const store = useTreeStore();
const lib = useLibraryStore();
const { currentDirId } = storeToRefs(store);
const { currentLibId } = storeToRefs(lib);

// 虚拟根（库名）与第一层目录默认展开
const isVirtualRoot = props.isRoot === true;
const expanded = ref(props.node.depth <= 0);

function fmtBytes(b: number): string {
  if (b > 1e9) return (b / 1e9).toFixed(1) + " GB";
  if (b > 1e6) return (b / 1e6).toFixed(1) + " MB";
  if (b > 1e3) return (b / 1e3).toFixed(0) + " KB";
  return b + " B";
}

function onClickName() {
  if (isVirtualRoot) {
    // 点击库名：显示整库所有文件
    if (currentLibId.value !== null) {
      store.selectLibraryRoot(currentLibId.value);
    }
    return;
  }
  store.selectDirectory(props.node.id);
}

// 缩进：虚拟根不缩进；真实节点相对库根再缩进（每层 14px），
// 这样库根(0)→一级目录(14)→二级(28)... 层级一目了然
const indentPx = isVirtualRoot ? 4 : (props.node.depth + 1) * 14;
</script>

<template>
  <div>
    <div
      class="flex items-center gap-1 py-1 cursor-pointer rounded hover:bg-slate-700/50 text-sm"
      :class="[
        (isVirtualRoot ? currentDirId === -1 : node.id === currentDirId)
          ? 'bg-sky-600/30'
          : '',
        isVirtualRoot ? 'font-semibold text-sky-200' : 'text-slate-200',
      ]"
      :style="{ paddingLeft: indentPx + 'px', paddingRight: '8px' }"
    >
      <!-- 展开/折叠箭头（有子节点才显示）-->
      <span
        v-if="node.children.length > 0"
        class="text-xs text-slate-400 w-4 inline-block select-none text-center"
        @click.stop="expanded = !expanded"
      >{{ expanded ? '▼' : '▶' }}</span>
      <span v-else class="inline-block w-4"></span>
      <!-- 文件夹图标 -->
      <span class="text-base leading-none">{{
        isVirtualRoot ? '🗃️' : (expanded && node.children.length ? '📂' : '📁')
      }}</span>
      <!-- 名称（点击显示该文件夹内容）-->
      <span
        class="flex-1 truncate"
        :title="isVirtualRoot ? node.name : node.path"
        @click="onClickName"
      >{{ node.name }}</span>
      <!-- 统计（直接含的文件数，提示该文件夹本身的内容量）-->
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
