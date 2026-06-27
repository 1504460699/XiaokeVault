import { createI18n } from "vue-i18n";
import zh from "./zh";
import en from "./en";

// 从 localStorage 读上次选择的语言，默认中文
function loadLocale(): "zh" | "en" {
  const saved = localStorage.getItem("vault.locale");
  if (saved === "en" || saved === "zh") return saved;
  return "zh"; // 默认中文
}

const i18n = createI18n({
  legacy: false, // Composition API 模式
  locale: loadLocale(),
  fallbackLocale: "zh",
  messages: { zh, en },
});

export default i18n;

/// 切换语言并持久化
export function setLocale(locale: "zh" | "en") {
  i18n.global.locale.value = locale;
  localStorage.setItem("vault.locale", locale);
}

/// 获取当前语言
export function getLocale(): "zh" | "en" {
  return i18n.global.locale.value as "zh" | "en";
}
