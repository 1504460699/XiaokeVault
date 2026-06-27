<script setup lang="ts">
import { ref, onMounted } from "vue";
import { storeToRefs } from "pinia";
import { useTypesStore } from "../stores/typesStore";
import type { AssetType } from "../types/library";

const props = defineProps<{ show: boolean }>();
const emit = defineEmits<{ close: [] }>();

const store = useTypesStore();
const { types, loading } = storeToRefs(store);

const editing = ref<AssetType | null>(null);
const extInput = ref("");
const reclassifyMsg = ref("");
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

onMounted(() => store.load());

function startEdit(t: AssetType) {
  editing.value = { ...t };
  extInput.value = t.extensions.join(", ");
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
    alert("类型标识和显示名必填");
    return;
  }
  const exts = extInput.value
    .split(",")
    .map((s) => s.trim().toLowerCase())
    .filter(Boolean);
  await store.upsert({
    kind: editing.value.kind,
    label: editing.value.label,
    extensions: exts,
    viewer: editing.value.viewer,
    is_source: editing.value.is_source,
    built_in: isExisting.value,
  });
  editing.value = null;
}

async function remove(kind: string) {
  if (!confirm(`删除类型 ${kind}？`)) return;
  try {
    await store.remove(kind);
  } catch (e) {
    alert(String(e));
  }
}

async function onReclassify() {
  const r = await store.reclassify();
  reclassifyMsg.value = `已重新分类 ${r.updated} 个文件`;
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
          <h2 class="text-lg font-bold flex-1">资源类型管理</h2>
          <button
            class="px-3 py-1 rounded bg-sky-600 hover:bg-sky-500 text-sm"
            @click="startNew"
          >
            + 新增类型
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
              placeholder="类型标识(如 video)"
              class="flex-1 bg-slate-700 rounded px-2 py-1 text-sm"
              :disabled="isExisting"
            />
            <input
              v-model="editing.label"
              placeholder="显示名(如 视频)"
              class="flex-1 bg-slate-700 rounded px-2 py-1 text-sm"
            />
          </div>
          <input
            v-model="extInput"
            placeholder="扩展名，逗号分隔(如 webm,mp4)"
            class="w-full bg-slate-700 rounded px-2 py-1 text-sm"
          />
          <div class="flex items-center gap-2 text-sm">
            <span class="text-slate-400">预览器：</span>
            <select v-model="editing.viewer" class="bg-slate-700 rounded px-2 py-1 text-sm">
              <option v-for="v in VIEWERS" :key="v" :value="v">{{ v }}</option>
            </select>
            <label class="flex items-center gap-1 ml-2">
              <input type="checkbox" v-model="editing.is_source" /> 源文件
            </label>
          </div>
          <div class="flex gap-2">
            <button
              class="px-3 py-1 rounded bg-emerald-600 hover:bg-emerald-500 text-sm"
              @click="save"
            >
              保存
            </button>
            <button
              class="px-3 py-1 rounded bg-slate-600 hover:bg-slate-500 text-sm"
              @click="editing = null"
            >
              取消
            </button>
          </div>
        </div>

        <!-- 类型列表 -->
        <div class="flex-1 overflow-auto">
          <table class="w-full text-sm">
            <thead class="text-xs text-slate-400 sticky top-0 bg-slate-800">
              <tr>
                <th class="text-left p-1">标识</th>
                <th class="text-left p-1">显示名</th>
                <th class="text-left p-1">扩展名</th>
                <th class="text-left p-1">预览器</th>
                <th class="text-left p-1">操作</th>
              </tr>
            </thead>
            <tbody>
              <tr
                v-for="t in types"
                :key="t.kind"
                class="border-t border-slate-700"
              >
                <td class="p-1 font-mono text-sky-300">{{ t.kind }}</td>
                <td class="p-1">{{ t.label }}</td>
                <td class="p-1 text-xs text-slate-400">
                  {{ t.extensions.join(", ") || "—" }}
                </td>
                <td class="p-1 text-xs">{{ t.viewer }}</td>
                <td class="p-1 whitespace-nowrap">
                  <button
                    class="px-2 py-0.5 rounded bg-slate-600 hover:bg-slate-500 text-xs mr-1"
                    @click="startEdit(t)"
                  >
                    编辑
                  </button>
                  <button
                    class="px-2 py-0.5 rounded bg-red-700 hover:bg-red-600 text-xs"
                    @click="remove(t.kind)"
                  >
                    删
                  </button>
                </td>
              </tr>
            </tbody>
          </table>
          <div v-if="loading" class="text-center text-slate-500 py-4 text-sm">
            加载中…
          </div>
        </div>

        <!-- 重新分类 -->
        <div class="flex items-center gap-3 mt-3 pt-3 border-t border-slate-700">
          <button
            class="px-3 py-1 rounded bg-amber-700 hover:bg-amber-600 text-sm"
            @click="onReclassify"
          >
            按新类型重新分类全库
          </button>
          <span v-if="reclassifyMsg" class="text-xs text-emerald-400">{{
            reclassifyMsg
          }}</span>
          <span class="text-xs text-slate-500 ml-auto">修改类型后需重新分类才生效</span>
        </div>

        <div class="flex justify-end mt-3">
          <button
            class="px-4 py-1 rounded bg-slate-600 hover:bg-slate-500 text-sm"
            @click="emit('close')"
          >
            关闭
          </button>
        </div>
      </div>
    </div>
  </Teleport>
</template>
