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

// 当前库的文件总数 + 总大小（聚合所有目录），显示在库根节点上
const libSummary = computed(() => {
  let count = 0;
  let bytes = 0;
  const walk = (nodes: typeof tree.value) => {
    for (const n of nodes) {
      count += n.file_count;
      bytes += n.total_bytes;
      if (n.children?.length) walk(n.children);
    }
  };
  walk(tree.value);
  return { count, bytes };
});

// 把当前库的目录树包进一个虚拟根节点（库名），点击它显示全库文件
const root = computed(() => ({
  id: -1, // 虚拟根，用负 id 避免与真实目录冲突
  name: libName.value,
  path: "",
  depth: -1, // 最顶层
  file_count: libSummary.value.count,
  total_bytes: libSummary.value.bytes,
  children: tree.value,
}));
</script>

<template>
  <aside class="w-64 shrink-0 overflow-y-auto bg-slate-800/50 border-r border-slate-700">
    <div v-if="tree.length === 0" class="px-3 py-4 text-sm text-slate-500">
      无目录。请先扫描库。
    </div>
    <div v-else class="py-2">
      <DirTreeNode :node="root" :is-root="true" />
    </div>
  </aside>
</template>
