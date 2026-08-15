import { createApp } from "vue";
import App from "./App.vue";
import "./style.css";

// 兜底错误提示：页面 JS 出错时在底部显示红字，方便定位白屏问题
function showFatalError(message: string) {
  if (!document.body) return;
  const el = document.createElement("div");
  el.style.cssText =
    "position:fixed;left:0;right:0;bottom:0;z-index:999999;background:#c0392b;color:#fff;font:12px/1.5 monospace;padding:8px 12px;white-space:pre-wrap;word-break:break-all;";
  el.textContent = `[Harnex 错误] ${message}`;
  document.body.appendChild(el);
}

window.addEventListener("error", (e) =>
  showFatalError(e.error?.stack || e.message),
);
window.addEventListener("unhandledrejection", (e) =>
  showFatalError(e.reason?.stack || String(e.reason)),
);

createApp(App).mount("#app");
