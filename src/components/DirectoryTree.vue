<script setup lang="ts">
import { storeToRefs } from "pinia";
import { useTreeStore } from "../stores/treeStore";
import DirTreeNode from "./DirTreeNode.vue";

const store = useTreeStore();
const { tree } = storeToRefs(store);
</script>

<template>
  <aside class="w-64 shrink-0 overflow-y-auto bg-slate-800/50 border-r border-slate-700">
    <div v-if="tree.length === 0" class="px-3 py-4 text-sm text-slate-500">
      无目录。请先扫描库。
    </div>
    <div v-else class="py-2">
      <DirTreeNode
        v-for="node in tree"
        :key="node.id"
        :node="node"
      />
    </div>
  </aside>
</template>
