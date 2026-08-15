<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  api,
  onConfigChanged,
  onDshTheme,
  onStatus,
  onToggleControlPanel,
  type AppConfig,
  type DshStatus,
} from "./lib/ipc";
import CaptionIcon from "./components/CaptionIcon.vue";
import ControlPanel from "./components/ControlPanel.vue";

const status = ref<DshStatus | null>(null);
const config = ref<AppConfig | null>(null);
const panelOpen = ref(false);
const iframeSrc = ref("");

const running = computed(() => status.value?.kind === "running");
const appWindow = getCurrentWindow();
const isMaximized = ref(false);
const darkMedia = window.matchMedia("(prefers-color-scheme: dark)");
const themePreference = ref<"system" | "light" | "dark">("system");

function applyTheme() {
  const resolved =
    themePreference.value === "system"
      ? darkMedia.matches
        ? "dark"
        : "light"
      : themePreference.value;
  document.documentElement.dataset.theme = resolved;
}

applyTheme();

async function refreshMaximized() {
  isMaximized.value = await appWindow.isMaximized();
}

function minimize() {
  void appWindow.minimize();
}

function maximize() {
  void appWindow.toggleMaximize().then(refreshMaximized);
}

function hideWindow() {
  void appWindow.hide();
}

const STATE_LABELS: Record<string, string> = {
  running: "运行中",
  starting: "启动中",
  stopped: "已停止",
  portBusy: "端口被占",
  error: "异常",
};
const stateLabel = computed(
  () => STATE_LABELS[status.value?.kind ?? "stopped"] ?? "已停止",
);
const stateCls = computed(
  () =>
    ({
      running: "ok",
      starting: "busy",
      stopped: "off",
      portBusy: "warn",
      error: "err",
    })[status.value?.kind ?? "stopped"] ?? "off",
);

watch(
  running,
  (isRunning) => {
    iframeSrc.value = isRunning
      ? `http://127.0.0.1:${config.value?.port ?? 3080}`
      : "";
  },
  { immediate: true },
);

let unlisteners: Array<() => void> = [];
let timer: number | undefined;

async function refresh() {
  try {
    status.value = await api.getStatus();
  } catch (e) {
    console.error("获取状态失败", e);
  }
}

onMounted(async () => {
  try {
    config.value = await api.getConfig();
  } catch (e) {
    console.error("读取配置失败", e);
  }
  await refresh();
  await refreshMaximized();
  timer = window.setInterval(refresh, 3000);
  unlisteners.push(
    await onStatus((s) => {
      status.value = s;
    }),
  );
  unlisteners.push(
    await onConfigChanged((c) => {
      config.value = c;
    }),
  );
  unlisteners.push(await onToggleControlPanel(() => (panelOpen.value = !panelOpen.value)));
  unlisteners.push(await appWindow.onResized(refreshMaximized));
  unlisteners.push(
    await onDshTheme((t) => {
      if (["system", "light", "dark"].includes(t.preference)) {
        themePreference.value = t.preference as "system" | "light" | "dark";
      }
      applyTheme();
    }),
  );
  darkMedia.addEventListener("change", applyTheme);
});

onUnmounted(() => {
  if (timer) clearInterval(timer);
  unlisteners.forEach((u) => u());
  darkMedia.removeEventListener("change", applyTheme);
});
</script>

<template>
  <div class="shell">
    <header class="toolbar" data-tauri-drag-region>
      <div class="left" data-tauri-drag-region>
        <span class="brand-dot" :class="stateCls"></span>
        <span class="brand">Harnex</span>
        <span class="state">{{ stateLabel }}</span>
      </div>
      <div class="right">
        <button
          class="tool-btn"
          :class="{ active: panelOpen }"
          @click="panelOpen = !panelOpen"
          title="控制台"
        >
          <span class="tool-dot" :class="stateCls"></span>
          <span>控制台</span>
        </button>
        <span class="sep"></span>
        <button class="cap-btn" title="最小化" @click="minimize">
          <CaptionIcon name="min" />
        </button>
        <button class="cap-btn" title="最大化 / 还原" @click="maximize">
          <CaptionIcon :name="isMaximized ? 'restore' : 'max'" />
        </button>
        <button class="cap-btn close" title="隐藏窗口" @click="hideWindow">
          <CaptionIcon name="close" />
        </button>
      </div>
    </header>
    <div class="stage">
      <iframe
        v-if="running && iframeSrc"
        :src="iframeSrc"
        class="dsh-frame"
        allow="clipboard-read; clipboard-write"
      ></iframe>
      <div v-else class="placeholder">
        <span class="dot"></span>
        <h1>Harnex</h1>
        <p>正在等待 DeepSeek Harness 启动…</p>
      </div>
      <ControlPanel
        v-if="panelOpen"
        :status="status"
        :config="config"
        @changed="refresh"
        @close="panelOpen = false"
      />
    </div>
  </div>
</template>

<style scoped>
.shell {
  height: 100%;
  display: flex;
  flex-direction: column;
  background: var(--bg);
}
.toolbar {
  height: 44px;
  flex: none;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 6px 0 14px;
  background: var(--panel);
  border-bottom: 1px solid var(--border);
  user-select: none;
}
.left {
  display: flex;
  align-items: center;
  gap: 8px;
}
.brand {
  font-weight: 600;
  font-size: 13px;
  letter-spacing: 0.2px;
}
.state {
  color: var(--muted);
  font-weight: 400;
  font-size: 12px;
}
.brand-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  flex: none;
  box-shadow: none;
}
.brand-dot.ok { background: var(--ok); }
.brand-dot.busy { background: var(--warn); animation: blink 1s infinite; }
.brand-dot.off { background: #4b4f57; }
.brand-dot.warn { background: var(--warn); }
.brand-dot.err { background: var(--err); }
@keyframes blink { 50% { opacity: 0.25; } }
.right { display: flex; gap: 4px; }
.tool-btn {
  display: flex;
  align-items: center;
  gap: 6px;
  border: none;
  background: var(--tool-btn-bg);
  color: var(--text);
  font-size: 12px;
  font-weight: 500;
  cursor: pointer;
  height: 26px;
  padding: 0 11px;
  border-radius: var(--radius-pill);
  line-height: 1;
  transition: background 0.15s ease, color 0.15s ease;
}
.tool-btn:hover { background: var(--tool-btn-hover); }
.tool-btn.active { background: var(--accent); color: #fff; }
.tool-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: var(--muted);
}
.tool-dot.ok { background: var(--ok); }
.tool-dot.busy { background: var(--warn); animation: blink 1s infinite; }
.tool-dot.off { background: var(--dot-off); }
.tool-dot.warn { background: var(--warn); }
.tool-dot.err { background: var(--err); }
.sep {
  width: 1px;
  height: 16px;
  background: var(--border-strong);
  margin: auto 5px;
}
.cap-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  border: none;
  background: transparent;
  color: var(--muted);
  cursor: pointer;
  width: 36px;
  height: 30px;
  border-radius: var(--radius-sm);
  transition: background 0.1s ease, color 0.1s ease;
}
.cap-btn:hover { background: var(--hover-soft); color: var(--text); }
.cap-btn:active { background: var(--hover-strong); }
.cap-btn.close:hover { background: var(--err); color: #fff; }
.cap-btn.close:active { background: #be1c2a; color: #fff; }
.stage {
  flex: 1;
  position: relative;
  min-height: 0;
}
.dsh-frame {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  border: none;
  background: var(--iframe-bg);
}
.placeholder {
  position: absolute;
  inset: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 8px;
  color: var(--muted);
}
.placeholder h1 {
  display: flex;
  align-items: center;
  gap: 10px;
  color: var(--text);
  font-size: 22px;
  letter-spacing: 1px;
  margin: 0;
}
.dot {
  width: 10px;
  height: 10px;
  border-radius: 50%;
  background: var(--warn);
  animation: blink 1.2s infinite;
  display: inline-block;
}
.placeholder p { margin: 0; font-size: 13px; }
</style>
