import { ref } from "vue";

// 轻量 Toast：全局单例，handleError 和组件都可直接调 show()
export interface ToastItem {
  id: number;
  message: string;
  type: "error" | "info" | "success";
}

const toasts = ref<ToastItem[]>([]);
let nextId = 1;

function show(message: string, type: ToastItem["type"] = "info", duration = 3500) {
  const id = nextId++;
  toasts.value.push({ id, message, type });
  setTimeout(() => {
    dismiss(id);
  }, duration);
}

function dismiss(id: number) {
  toasts.value = toasts.value.filter((t) => t.id !== id);
}

export function useToast() {
  return {
    toasts,
    show,
    dismiss,
    error: (msg: string) => show(msg, "error", 5000),
    info: (msg: string) => show(msg, "info"),
    success: (msg: string) => show(msg, "success"),
  };
}

/**
 * 统一错误处理：解析后端 AppError（{code, message}），按 code 决定提示方式。
 * - NotFound：静默（通常是空数据，不惊扰用户）
 * - 其他：Toast 弹出中文 message
 * 兼容旧式纯字符串错误。
 */
export function handleError(e: unknown, fallback = "操作失败，请重试") {
  // Tauri invoke 错误可能是 { code, message } 对象或字符串
  let code: string | undefined;
  let message: string | undefined;
  if (e && typeof e === "object") {
    code = (e as { code?: string }).code;
    message = (e as { message?: string }).message;
  } else if (typeof e === "string") {
    // 尝试解析 JSON（Tauri 有时把错误序列化成字符串）
    try {
      const parsed = JSON.parse(e);
      code = parsed.code;
      message = parsed.message;
    } catch {
      message = e;
    }
  }
  const msg = message ?? fallback;
  // NotFound 多为数据不存在（如切换库时的旧查询），静默不打扰
  if (code === "NotFound") {
    console.warn("[NotFound]", msg);
    return;
  }
  show(msg, "error", 5000);
}
