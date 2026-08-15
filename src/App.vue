<script setup lang="ts">
import { onMounted, onUnmounted, ref } from "vue";
import { api, onConfigChanged, onStatus, type AppConfig, type DshStatus } from "./lib/ipc";
import CommandBox from "./components/CommandBox.vue";
import ControlButtons from "./components/ControlButtons.vue";
import SettingsPanel from "./components/SettingsPanel.vue";
import StatusCard from "./components/StatusCard.vue";

const status = ref<DshStatus | null>(null);
const config = ref<AppConfig | null>(null);

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
});

onUnmounted(() => {
  if (timer) clearInterval(timer);
  unlisteners.forEach((u) => u());
});
</script>

<template>
  <div class="shell">
    <header class="titlebar" data-tauri-drag-region>
      <div class="brand" data-tauri-drag-region>
        <span class="brand-dot"></span>
        <span>Harnex</span>
        <span class="sub">DSH 桌面壳</span>
      </div>
      <div class="actions">
        <button class="icon-btn" title="显示主窗口" @click="api.showMain()">⛶</button>
        <button class="icon-btn" title="隐藏浮窗" @click="api.hideControl()">×</button>
      </div>
    </header>
    <main class="body">
      <StatusCard :status="status" :config="config" />
      <ControlButtons :status="status" @changed="refresh" />
      <CommandBox :work-dir="config?.workDir ?? ''" />
      <SettingsPanel :config="config" @saved="refresh" />
    </main>
  </div>
</template>

<style scoped>
.shell {
  height: 100%;
  display: flex;
  flex-direction: column;
}
.titlebar {
  height: 38px;
  flex: none;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 10px 0 14px;
  background: var(--panel);
  border-bottom: 1px solid var(--border);
  user-select: none;
}
.brand {
  display: flex;
  align-items: center;
  gap: 8px;
  font-weight: 700;
  font-size: 13px;
}
.brand-dot {
  width: 9px;
  height: 9px;
  border-radius: 50%;
  background: linear-gradient(135deg, #4f8cff, #37c98b);
}
.sub { color: var(--muted); font-weight: 400; font-size: 11px; }
.actions { display: flex; gap: 4px; }
.icon-btn {
  background: none;
  border: none;
  color: var(--muted);
  font-size: 14px;
  cursor: pointer;
  width: 26px;
  height: 26px;
  border-radius: 6px;
  line-height: 1;
}
.icon-btn:hover { background: var(--panel-2); color: var(--text); }
.body {
  flex: 1;
  overflow-y: auto;
  padding: 12px;
  display: flex;
  flex-direction: column;
  gap: 10px;
}
</style>
