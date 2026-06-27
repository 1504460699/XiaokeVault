<script setup lang="ts">
import { computed } from "vue";
import { storeToRefs } from "pinia";
import { useTreeStore } from "../stores/treeStore";
import { useLibraryStore } from "../stores/libraryStore";
import DirTreeNode from "./DirTreeNode.vue";

const store = useTreeStore();
const lib = useLibraryStore();
const { tree } = storeToRefs(store);
const { libraries, currentLibId } = storeToRefs(lib);

// 当前库名（作虚拟根节点）
const libName = computed(() => {
  const l = libraries.value.find((x) => x.id === currentLibId.value);
  return l?.name ?? "";
});

// 把当前库的目录树包进一个虚拟根节点（库名），点击它显示全库文件
const root = computed(() => ({
  id: -1, // 虚拟根，用负 id 避免与真实目录冲突
  name: libName.value,
  path: "",
  depth: -1, // 最顶层，DirTreeNode 默认展开 depth===0，这里用 -1 也展开
  file_count: 0,
  total_bytes: 0,
  children: tree.value,
}));
</script>

<template>
  <aside class="w-64 shrink-0 overflow-y-auto bg-slate-800/50 border-r border-slate-700">
    <div v-if="tree.length === 0" class="px-3 py-4 text-sm text-slate-500">
      无目录。请先扫描库。
    </div>
    <div v-else class="py-2">
      <DirTreeNode :node="root" />
    </div>
  </aside>
</template>
