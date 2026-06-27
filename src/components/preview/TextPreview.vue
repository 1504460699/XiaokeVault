<script setup lang="ts">
import { ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { getFileUrl } from "../../ipc/fileUrl";
import type { FileNode } from "../../types/library";

const { t } = useI18n();
const props = defineProps<{ file: FileNode }>();
const content = ref("");
const loading = ref(false);
const error = ref<string | null>(null);
const TRUNCATE_BYTES = 50000;

watch(
  () => props.file,
  async (f) => {
    loading.value = true;
    error.value = null;
    content.value = "";
    try {
      const url = getFileUrl(f);
      const res = await fetch(url);
      if (!res.ok) throw new Error("HTTP " + res.status);
      const text = await res.text();
      content.value =
        text.length > TRUNCATE_BYTES
          ? text.slice(0, TRUNCATE_BYTES) +
            `\n\n${t("preview.truncated", { n: text.length })}`
          : text;
    } catch (e) {
      error.value = String(e);
    } finally {
      loading.value = false;
    }
  },
  { immediate: true },
);
</script>

<template>
  <div class="w-full text-xs text-slate-300 font-mono">
    <div v-if="loading" class="text-slate-500">{{ t("common.loading") }}</div>
    <div v-else-if="error" class="text-red-400">{{ t("errors.loadFailed", { msg: error }) }}</div>
    <pre
      v-else
      class="whitespace-pre-wrap break-all bg-slate-900 p-2 rounded w-full"
      >{{ content }}</pre
    >
  </div>
</template>
