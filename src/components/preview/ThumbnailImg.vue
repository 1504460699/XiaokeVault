<script setup lang="ts">
import { ref, watch, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { convertFileSrc } from "@tauri-apps/api/core";
import type { FileNode } from "../../types/library";

const props = defineProps<{ file: FileNode }>();

// 模块级缓存：file_id -> url，避免重复请求
const cache = new Map<number, string>();
const url = ref("");

async function load() {
  const fid = props.file.id;
  if (cache.has(fid)) {
    url.value = cache.get(fid)!;
    return;
  }
  try {
    const res = await invoke<{ path: string; is_thumb: boolean }>(
      "get_thumbnail",
      { fileId: fid },
    );
    const u = convertFileSrc(res.path);
    cache.set(fid, u);
    url.value = u;
  } catch {
    // 失败降级用原图
    url.value = convertFileSrc(props.file.abs_path);
  }
}

onMounted(load);
watch(() => props.file.id, load);
</script>

<template>
  <img
    v-if="url"
    :src="url"
    class="max-w-full max-h-full object-contain"
    loading="lazy"
  />
  <div v-else class="text-slate-600 text-xs">…</div>
</template>
