<script setup lang="ts">
import { ref, onMounted, onUnmounted } from "vue";
import { useI18n } from "vue-i18n";
import { storeToRefs } from "pinia";
import { useTypesStore } from "../stores/typesStore";
import { listen } from "@tauri-apps/api/event";
import type { AssetType } from "../types/library";

// 注意：函数参数也用 t（AssetType），故 i18n 用 tt 别名避免冲突
const { t: tt } = useI18n();
const props = defineProps<{ show: boolean }>();
const emit = defineEmits<{ close: [] }>();

const store = useTypesStore();
const { types, loading } = storeToRefs(store);

const editing = ref<AssetType | null>(null);
const extInput = ref("");
const reclassifyMsg = ref("");
const reclassifying = ref(false);
const progress = ref<{ done: number; total: number } | null>(null);
const isExisting = ref(false);

const VIEWERS = [
  "image",
  "animated",
  "vector",
  "audio",
  "font",
  "text",
  "3d",
  "binary-source",
  "fallback",
];

let unlisten: (() => void) | null = null;

onMounted(async () => {
  await store.load();
  unlisten = await listen<{ done: number; total: number }>(
    "reclassify://progress",
    (e) => {
      progress.value = e.payload;
    },
  );
});

onUnmounted(() => {
  unlisten?.();
});

function startEdit(tp: AssetType) {
  editing.value = { ...tp };
  extInput.value = tp.extensions.join(", ");
  isExisting.value = true;
}

function startNew() {
  editing.value = {
    kind: "",
    label: "",
    extensions: [],
    viewer: "fallback",
    icon: null,
    is_source: false,
  };
  extInput.value = "";
  isExisting.value = false;
}

async function save() {
  if (!editing.value) return;
  if (!editing.value.kind || !editing.value.label) {
    alert(tt("types.requiredAlert"));
    return;
  }
  const exts = extInput.value
    .split(",")
    .map((s) => s.trim().toLowerCase())
    .filter(Boolean);
  try {
    await store.upsert({
      kind: editing.value.kind,
      label: editing.value.label,
      extensions: exts,
      viewer: editing.value.viewer,
      is_source: editing.value.is_source,
      built_in: isExisting.value,
    });
    editing.value = null;
  } catch (e) {
    alert(tt("types.saveFailed", { msg: String(e) }));
  }
}

async function remove(kind: string) {
  if (!confirm(tt("types.deleteType", { name: kind }))) return;
  try {
    await store.remove(kind);
  } catch (e) {
    alert(String(e));
  }
}

async function onReclassify() {
  reclassifying.value = true;
  progress.value = { done: 0, total: 0 };
  reclassifyMsg.value = "";
  try {
    const r = await store.reclassify();
    reclassifyMsg.value = tt("types.reclassifyDone", { n: r.updated });
  } catch (e) {
    reclassifyMsg.value = tt("types.reclassifyFailed", { msg: String(e) });
  } finally {
    reclassifying.value = false;
    progress.value = null;
  }
}
</script>

<template>
  <Teleport to="body">
    <div
      v-if="props.show"
      class="fixed inset-0 bg-black/60 flex items-center justify-center z-50"
      @click.self="emit('close')"
    >
      <div
        class="bg-slate-800 rounded-lg p-6 w-[680px] max-h-[85vh] flex flex-col text-slate-100"
      >
        <div class="flex items-center mb-3">
          <h2 class="text-lg font-bold flex-1">{{ tt("types.title") }}</h2>
          <button
            class="px-3 py-1 rounded bg-sky-600 hover:bg-sky-500 text-sm"
            @click="startNew"
          >
            {{ tt("types.addType") }}
          </button>
        </div>

        <!-- 编辑区 -->
        <div
          v-if="editing"
          class="bg-slate-900 rounded p-3 mb-3 border border-slate-700 space-y-2"
        >
          <div class="flex gap-2">
            <input
              v-model="editing.kind"
              :placeholder="tt('types.kindPlaceholder')"
              class="flex-1 bg-slate-700 rounded px-2 py-1 text-sm"
              :disabled="isExisting"
            />
            <input
              v-model="editing.label"
              :placeholder="tt('types.namePlaceholder')"
              class="flex-1 bg-slate-700 rounded px-2 py-1 text-sm"
            />
          </div>
          <input
            v-model="extInput"
            :placeholder="tt('types.extsPlaceholder')"
            class="w-full bg-slate-700 rounded px-2 py-1 text-sm"
          />
          <div class="flex items-center gap-2 text-sm">
            <span class="text-slate-400">{{ tt("types.viewerLabel") }}</span>
            <select v-model="editing.viewer" class="bg-slate-700 rounded px-2 py-1 text-sm">
              <option v-for="v in VIEWERS" :key="v" :value="v">{{ v }}</option>
            </select>
            <label class="flex items-center gap-1 ml-2">
              <input type="checkbox" v-model="editing.is_source" /> {{ tt("types.sourceFile") }}
            </label>
          </div>
          <div class="flex gap-2">
            <button
              class="px-3 py-1 rounded bg-emerald-600 hover:bg-emerald-500 text-sm"
              @click="save"
            >
              {{ tt("common.save") }}
            </button>
            <button
              class="px-3 py-1 rounded bg-slate-600 hover:bg-slate-500 text-sm"
              @click="editing = null"
            >
              {{ tt("common.cancel") }}
            </button>
          </div>
        </div>

        <!-- 类型列表 -->
        <div class="flex-1 overflow-auto">
          <table class="w-full text-sm">
            <thead class="text-xs text-slate-400 sticky top-0 bg-slate-800">
              <tr>
                <th class="text-left p-1">{{ tt("types.kind") }}</th>
                <th class="text-left p-1">{{ tt("types.name") }}</th>
                <th class="text-left p-1">{{ tt("types.exts") }}</th>
                <th class="text-left p-1">{{ tt("types.viewer") }}</th>
                <th class="text-left p-1">{{ tt("types.operation") }}</th>
              </tr>
            </thead>
            <tbody>
              <tr
                v-for="tp in types"
                :key="tp.kind"
                class="border-t border-slate-700"
              >
                <td class="p-1 font-mono text-sky-300">{{ tp.kind }}</td>
                <td class="p-1">{{ tp.label }}</td>
                <td class="p-1 text-xs text-slate-400">
                  {{ tp.extensions.join(", ") || "—" }}
                </td>
                <td class="p-1 text-xs">{{ tp.viewer }}</td>
                <td class="p-1 whitespace-nowrap">
                  <button
                    class="px-2 py-0.5 rounded bg-slate-600 hover:bg-slate-500 text-xs mr-1 whitespace-nowrap"
                    @click="startEdit(tp)"
                  >
                    {{ tt("types.editType") }}
                  </button>
                  <button
                    class="px-2 py-0.5 rounded bg-red-700 hover:bg-red-600 text-xs whitespace-nowrap"
                    @click="remove(tp.kind)"
                  >
                    {{ tt("common.delete") }}
                  </button>
                </td>
              </tr>
            </tbody>
          </table>
          <div v-if="loading" class="text-center text-slate-500 py-4 text-sm">
            {{ tt("common.loading") }}
          </div>
        </div>

        <!-- 重新分类 -->
        <div class="mt-3 pt-3 border-t border-slate-700">
          <div class="flex items-center gap-3 mb-2">
            <button
              class="px-3 py-1 rounded bg-amber-700 hover:bg-amber-600 disabled:opacity-50 text-sm"
              :disabled="reclassifying"
              @click="onReclassify"
            >
              {{ reclassifying ? tt("types.reclassifying") : tt("types.reclassify") }}
            </button>
            <span v-if="reclassifyMsg" class="text-xs text-emerald-400">{{
              reclassifyMsg
            }}</span>
            <span class="text-xs text-slate-500 ml-auto">{{ tt("types.needReclassify") }}</span>
          </div>
          <div v-if="progress && progress.total > 0" class="flex items-center gap-2 text-xs text-slate-400">
            <div class="flex-1 bg-slate-700 rounded h-1.5 overflow-hidden">
              <div
                class="bg-amber-500 h-full transition-all"
                :style="{ width: (progress.done / progress.total) * 100 + '%' }"
              ></div>
            </div>
            <span>{{ progress.done }} / {{ progress.total }}</span>
          </div>
        </div>

        <div class="flex justify-end mt-3">
          <button
            class="px-4 py-1 rounded bg-slate-600 hover:bg-slate-500 text-sm"
            @click="emit('close')"
          >
            {{ tt("common.close") }}
          </button>
        </div>
      </div>
    </div>
  </Teleport>
</template>
