<script setup lang="ts">
import { ref } from "vue";
import type { AppConfig, DshStatus } from "../lib/ipc";
import { api } from "../lib/ipc";
import { getVersion } from "@tauri-apps/api/app";
import CommandBox from "./CommandBox.vue";
import ControlButtons from "./ControlButtons.vue";
import SettingsPanel from "./SettingsPanel.vue";
import StatusCard from "./StatusCard.vue";
import { t } from "../lib/i18n";

defineProps<{
  status: DshStatus | null;
  config: AppConfig | null;
  winId: number;
  zoom: number;
}>();

const emit = defineEmits<{
  changed: [];
  close: [];
  "zoom-in": [];
  "zoom-out": [];
  "zoom-reset": [];
}>();

const aboutOpen = ref(false);
const version = ref("");
const GITHUB_URL = "https://github.com/YgtqRun/Harnex";

async function toggleAbout() {
  aboutOpen.value = !aboutOpen.value;
  if (aboutOpen.value && !version.value) {
    try {
      version.value = await getVersion();
    } catch {
      version.value = "0.1.0";
    }
  }
}

function openGithub() {
  api.openUrl(GITHUB_URL).catch((e) => alert(String(e)));
}
</script>

<template>
  <section class="panel">
    <header class="head">
      <span class="title">Harnex {{ t("console") }}</span>
      <button class="close" :title="t('console')" @click="emit('close')">
        <svg viewBox="0 0 16 16" width="12" height="12" aria-hidden="true">
          <path
            d="M3.2 3.2l9.6 9.6M12.8 3.2L3.2 12.8"
            stroke="currentColor"
            stroke-width="1.6"
            stroke-linecap="round"
          />
        </svg>
      </button>
    </header>
    <div class="body">
      <StatusCard :status="status" :config="config" />
      <ControlButtons :status="status" @changed="emit('changed')" />
      <div class="zoom-row">
        <span class="zoom-label">{{ t("zoom") }}</span>
        <button class="zoom-btn" :title="t('zoomOut')" @click="emit('zoom-out')">−</button>
        <span class="zoom-value">{{ Math.round(zoom * 100) }}%</span>
        <button class="zoom-btn" :title="t('zoomIn')" @click="emit('zoom-in')">+</button>
        <button class="zoom-btn reset" :disabled="zoom === 1" @click="emit('zoom-reset')">
          {{ t("reset") }}
        </button>
      </div>
      <CommandBox :win-id="winId" :work-dir="config?.workDir ?? ''" />
      <SettingsPanel :config="config" @saved="emit('changed')" />
      <div class="info-section">
        <button class="toggle" @click="toggleAbout">
          <span>{{ t("about") }}</span>
          <span class="chevron" :class="{ open: aboutOpen }">▾</span>
        </button>
        <div v-if="aboutOpen" class="info-grid">
          <div class="info-row">
            <span class="k">{{ t("harnexVersion") }}</span>
            <span class="v">{{ version }}</span>
          </div>
          <div class="info-row">
            <span class="k">{{ t("github") }}</span>
            <button class="gh" @click="openGithub">{{ t("githubOpen") }} ↗</button>
          </div>
        </div>
      </div>
    </div>
  </section>
</template>

<style scoped>
.panel {
  position: absolute;
  top: 0;
  right: 0;
  bottom: 0;
  width: 380px;
  display: flex;
  flex-direction: column;
  background: var(--panel);
  border-left: 1px solid var(--border);
  z-index: 20;
}
.head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 10px 10px 16px;
}
.title {
  font-size: 13px;
  font-weight: 600;
  letter-spacing: 0.2px;
}
.close {
  display: flex;
  align-items: center;
  justify-content: center;
  background: none;
  border: none;
  color: var(--muted);
  cursor: pointer;
  width: 26px;
  height: 26px;
  border-radius: 6px;
}
.close:hover { background: var(--hover-soft); color: var(--text); }
.body {
  overflow-y: auto;
  padding: 0 14px 14px;
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.zoom-row {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 8px 0;
  border-top: 1px solid var(--border);
  border-bottom: 1px solid var(--border);
}
.zoom-label {
  font-size: 11px;
  color: var(--muted);
  margin-right: 2px;
}
.zoom-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  min-width: 24px;
  height: 24px;
  padding: 0 8px;
  border-radius: 6px;
  border: 1px solid var(--border);
  background: transparent;
  color: var(--text);
  font-size: 12px;
  cursor: pointer;
  transition: background 0.15s ease, border-color 0.15s ease;
}
.zoom-btn:hover:not(:disabled) {
  background: var(--hover-soft);
  border-color: var(--border-strong);
}
.zoom-btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}
.zoom-value {
  min-width: 44px;
  text-align: center;
  font-size: 12px;
  font-variant-numeric: tabular-nums;
}
.zoom-btn.reset {
  margin-left: auto;
  font-size: 11px;
  color: var(--muted);
}
.zoom-btn.reset:hover:not(:disabled) { color: var(--text); }
.info-section {
  border-top: 1px solid var(--border);
  padding-top: 8px;
}
.toggle {
  display: flex;
  align-items: center;
  gap: 6px;
  background: none;
  border: none;
  color: var(--muted);
  cursor: pointer;
  font-size: 11px;
  padding: 0;
}
.toggle:hover { color: var(--text); }
.chevron {
  font-size: 10px;
  transition: transform 0.15s ease;
}
.chevron.open { transform: rotate(180deg); }
.info-grid {
  margin-top: 10px;
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.info-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  font-size: 11px;
}
.k { color: var(--muted); }
.v {
  color: var(--text);
  font-family: var(--font-mono);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.gh {
  background: none;
  border: none;
  color: var(--accent);
  cursor: pointer;
  font-size: 11px;
  padding: 0;
}
.gh:hover { text-decoration: underline; }
</style>
