<script setup lang="ts">
import { useToast } from "../utils/toast";

const { toasts, dismiss } = useToast();
</script>

<template>
  <Teleport to="body">
    <div class="fixed top-4 left-1/2 -translate-x-1/2 z-[100] flex flex-col gap-2 items-center pointer-events-none">
      <div
        v-for="t in toasts"
        :key="t.id"
        class="px-4 py-2 rounded-lg shadow-lg text-sm pointer-events-auto max-w-[80vw] flex items-center gap-2"
        :class="{
          'bg-red-600 text-white': t.type === 'error',
          'bg-sky-600 text-white': t.type === 'info',
          'bg-emerald-600 text-white': t.type === 'success',
        }"
        @click="dismiss(t.id)"
      >
        <span class="whitespace-pre-wrap">{{ t.message }}</span>
        <span class="opacity-70 cursor-pointer">✕</span>
      </div>
    </div>
  </Teleport>
</template>
