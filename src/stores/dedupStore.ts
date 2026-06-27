import { defineStore } from "pinia";
import { ref } from "vue";
import { dedupIpc } from "../ipc/dedup";
import { handleError } from "../utils/toast";
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
    } catch (e) {
      handleError(e, "去重检测失败");
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
    } catch (e) {
      handleError(e, "清理失败");
      throw e;
    } finally {
      removing.value = false;
    }
  }

  async function dismissGroup(groupId: number) {
    await dedupIpc.dismissGroup(groupId);
    groups.value = groups.value.filter((g) => g.id !== groupId);
  }

  return { groups, report, scanning, removing, runDedup, loadGroups, removeMember, removeAll, dismissGroup };
});
