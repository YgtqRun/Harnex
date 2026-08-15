<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  api,
  onConfigChanged,
  onDshTheme,
  onStatus,
  type AppConfig,
  type DshStatus,
} from "./lib/ipc";
import CaptionIcon from "./components/CaptionIcon.vue";
import ControlPanel from "./components/ControlPanel.vue";
import { applyLocale, onDshLocale, t } from "./lib/i18n";

const status = ref<DshStatus | null>(null);
const config = ref<AppConfig | null>(null);
const panelOpen = ref(false);
const iframeSrc = ref("");

const winId = Number(new URLSearchParams(window.location.search).get("id")) || 0;
const running = computed(() => status.value?.kind === "running");
const appWindow = getCurrentWindow();
const isMaximized = ref(false);
const zoom = ref<number>(
  Number(localStorage.getItem(`harnex.zoom.${winId}`)) || 1,
);
const zoomStyle = computed(() => ({ "--zoom": String(zoom.value) }));

function clampZoom(v: number) {
  return Math.min(2, Math.max(0.5, Math.round(v * 10) / 10));
}

function zoomIn() {
  zoom.value = clampZoom(zoom.value + 0.1);
}

function zoomOut() {
  zoom.value = clampZoom(zoom.value - 0.1);
}

function zoomReset() {
  zoom.value = 1;
}

watch(zoom, (v) => {
  localStorage.setItem(`harnex.zoom.${winId}`, String(v));
});

function onKeydown(e: KeyboardEvent) {
  if (!e.ctrlKey) return;
  if (e.key === "=" || e.key === "+") {
    e.preventDefault();
    zoomIn();
  } else if (e.key === "-") {
    e.preventDefault();
    zoomOut();
  } else if (e.key === "0") {
    e.preventDefault();
    zoomReset();
  }
}

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

function closeWindow() {
  void appWindow.close();
}

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
  window.addEventListener("keydown", onKeydown);
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
  unlisteners.push(await appWindow.onResized(refreshMaximized));
  unlisteners.push(await onDshLocale((pref) => applyLocale(pref)));
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
  window.removeEventListener("keydown", onKeydown);
});
</script>

<template>
  <div class="shell">
    <header class="topbar" data-tauri-drag-region>
      <span class="brand">Harnex</span>
      <div class="bar-right">
        <button class="f-btn" :title="t('newWindow')" @click="api.newWindow()">
          <span class="plus">＋</span>
          <span>{{ t("newWindow") }}</span>
        </button>
        <button
          class="f-btn"
          :class="{ active: panelOpen }"
          @click="panelOpen = !panelOpen"
          :title="t('console')"
        >
          <span class="tool-dot" :class="stateCls"></span>
          <span>{{ t("console") }}</span>
        </button>
        <span class="sep"></span>
        <button class="cap-btn" :title="t('minimize')" @click="minimize">
          <CaptionIcon name="min" />
        </button>
        <button class="cap-btn" :title="t('maximize')" @click="maximize">
          <CaptionIcon :name="isMaximized ? 'restore' : 'max'" />
        </button>
        <button class="cap-btn close" :title="t('closeWindow')" @click="closeWindow">
          <CaptionIcon name="close" />
        </button>
      </div>
    </header>
    <div class="stage">
      <div v-if="running && iframeSrc" class="frame">
        <iframe
          :src="iframeSrc"
          class="dsh-frame"
          :style="zoomStyle"
          allow="clipboard-read; clipboard-write"
        ></iframe>
      </div>
      <div v-else class="placeholder">
        <span class="dot"></span>
        <h1>Harnex</h1>
        <p>{{ t("waiting") }}</p>
      </div>
      <div v-if="panelOpen" class="backdrop" @click="panelOpen = false"></div>
      <ControlPanel
        v-if="panelOpen"
        :status="status"
        :config="config"
        :win-id="winId"
        :zoom="zoom"
        @changed="refresh"
        @close="panelOpen = false"
        @zoom-in="zoomIn"
        @zoom-out="zoomOut"
        @zoom-reset="zoomReset"
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
.topbar {
  height: 36px;
  flex: none;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 8px 0 14px;
  background: var(--bg);
  border-bottom: 1px solid var(--border);
  user-select: none;
}
.brand {
  font-size: 13px;
  font-weight: 600;
  letter-spacing: 0.2px;
  color: var(--text);
}
.bar-right {
  display: flex;
  align-items: center;
  gap: 2px;
}
.stage {
  flex: 1;
  position: relative;
  min-height: 0;
}
.f-btn {
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
  padding: 0 10px;
  border-radius: 8px;
  line-height: 1;
  transition: background 0.15s ease, color 0.15s ease;
}
.f-btn:hover { background: var(--tool-btn-hover); }
.f-btn.active { background: var(--accent); color: #fff; }
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
@keyframes blink { 50% { opacity: 0.25; } }
.plus {
  font-size: 13px;
  line-height: 1;
}
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
.frame {
  position: absolute;
  inset: 0;
  overflow: hidden;
}
.backdrop {
  position: absolute;
  inset: 0;
  z-index: 15;
}
.dsh-frame {
  width: calc(100% / var(--zoom, 1));
  height: calc(100% / var(--zoom, 1));
  transform: scale(var(--zoom, 1));
  transform-origin: 0 0;
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
