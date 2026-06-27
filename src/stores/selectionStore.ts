import { defineStore } from "pinia";
import { ref } from "vue";

// M2 仅占位：选中预览的文件。M3 扩展为导出勾选状态。
export const useSelectionStore = defineStore("selection", () => {
  const previewFileId = ref<number | null>(null);
  function setPreview(id: number | null) {
    previewFileId.value = id;
  }
  return { previewFileId, setPreview };
});
