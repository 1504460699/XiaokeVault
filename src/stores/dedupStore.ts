import { defineStore } from "pinia";
import { ref } from "vue";
import { dedupIpc } from "../ipc/dedup";
import type { DupGroup, DedupReport } from "../types/dedup";

export const useDedupStore = defineStore("dedup", () => {
  const groups = ref<DupGroup[]>([]);
  const report = ref<DedupReport | null>(null);
  const scanning = ref(false);
  const removing = ref(false);

  async function runDedup(libId: number) {
    scanning.value = true;
    try {
      report.value = await dedupIpc.runDedup(libId);
      groups.value = await dedupIpc.getGroups();
    } finally {
      scanning.value = false;
    }
  }

  async function loadGroups() {
    groups.value = await dedupIpc.getGroups();
  }

  async function removeMember(fileId: number, groupId: number, backupRoot: string) {
    await dedupIpc.removeDuplicate(fileId, backupRoot);
    groups.value = groups.value.filter((g) => g.id !== groupId);
    if (report.value) {
      report.value = {
        ...report.value,
        groups: report.value.groups - 1,
        removable_files: Math.max(0, report.value.removable_files - 1),
      };
    }
  }

  async function removeAll(backupRoot: string) {
    removing.value = true;
    try {
      const r = await dedupIpc.removeAllDuplicates(backupRoot);
      groups.value = [];
      report.value = null;
      return r;
    } finally {
      removing.value = false;
    }
  }

  return { groups, report, scanning, removing, runDedup, loadGroups, removeMember, removeAll };
});
