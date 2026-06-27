import { defineStore } from "pinia";
import { ref } from "vue";
import { typesIpc } from "../ipc/types";
import type { AssetType } from "../types/library";

export const useTypesStore = defineStore("types", () => {
  const types = ref<AssetType[]>([]);
  const loading = ref(false);

  async function load() {
    loading.value = true;
    try {
      types.value = await typesIpc.list();
    } finally {
      loading.value = false;
    }
  }

  async function upsert(t: {
    kind: string;
    label: string;
    extensions: string[];
    viewer: string;
    is_source: boolean;
    built_in: boolean;
  }) {
    await typesIpc.upsert(t);
    await load();
  }

  async function remove(kind: string) {
    await typesIpc.remove(kind);
    await load();
  }

  async function reclassify() {
    return await typesIpc.reclassify();
  }

  return { types, loading, load, upsert, remove, reclassify };
});
