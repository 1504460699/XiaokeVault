import { defineStore } from "pinia";
import { ref } from "vue";

// M2 占位。M3 填充项目/勾选/导出任务。
export const useExportStore = defineStore("export", () => {
  const ready = ref(false);
  return { ready };
});
