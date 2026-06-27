import { createApp } from "vue";
import { createPinia } from "pinia";
import "./assets/main.css";
import App from "./App.vue";
import i18n from "./i18n";

const app = createApp(App);
app.use(createPinia());
app.use(i18n);
app.mount("#app");

// 禁用 webview 默认右键菜单（桌面应用无需浏览器右键）。
// 例外：输入框/文本域保留右键，方便复制/粘贴。
document.addEventListener("contextmenu", (e) => {
  const el = e.target as HTMLElement | null;
  const tag = el?.tagName;
  const isEditable =
    tag === "INPUT" ||
    tag === "TEXTAREA" ||
    el?.isContentEditable === true;
  if (!isEditable) {
    e.preventDefault();
  }
});
