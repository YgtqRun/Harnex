<script setup lang="ts">
import { computed, onUnmounted, ref, watch } from "vue";
import { api, type AppConfig, type DshStatus } from "../lib/ipc";
import { formatUptime, t } from "../lib/i18n";

const props = defineProps<{
  status: DshStatus | null;
  config: AppConfig | null;
}>();

const label = computed(() => {
  const key =
    ({
      running: "running",
      starting: "starting",
      stopped: "stopped",
      portBusy: "portBusy",
      error: "error",
    })[props.status?.kind ?? "stopped"] ?? "stopped";
  return t(key);
});
const cls = computed(
  () =>
    ({
      running: "ok",
      starting: "busy",
      stopped: "off",
      portBusy: "warn",
      error: "err",
    })[props.status?.kind ?? "stopped"] ?? "off",
);

const uptime = ref("");
const logOpen = ref(false);
const logLines = ref<string[]>([]);
let uptimeTimer: number | undefined;
let logTimer: number | undefined;

watch(
  () => props.status?.startedAt,
  (v) => {
    if (uptimeTimer) clearInterval(uptimeTimer);
    if (v) {
      const update = () => {
        const secs = Math.max(0, Math.floor((Date.now() - v) / 1000));
        uptime.value = formatUptime(secs);
      };
      update();
      uptimeTimer = window.setInterval(update, 1000);
    } else {
      uptime.value = "";
    }
  },
  { immediate: true },
);

async function loadLog() {
  logLines.value = await api.getDshLog();
}

function toggleLog() {
  logOpen.value = !logOpen.value;
  if (logOpen.value) {
    void loadLog();
    logTimer = window.setInterval(loadLog, 2000);
  } else if (logTimer) {
    clearInterval(logTimer);
    logTimer = undefined;
  }
}

onUnmounted(() => {
  if (uptimeTimer) clearInterval(uptimeTimer);
  if (logTimer) clearInterval(logTimer);
});
</script>

<template>
  <div class="status">
    <div class="status-main">
      <span class="dot" :class="cls"></span>
      <span class="name">{{ label }}</span>
      <span
        v-if="status?.message"
        class="msg"
        :class="{ warn: status?.kind === 'portBusy' || status?.kind === 'error' }"
      >
        {{ status.message }}
      </span>
    </div>
    <div class="meta">
      <span v-if="uptime" class="meta-item">{{ uptime }}</span>
      <span class="meta-item">{{ t("port") }} {{ status?.port ?? config?.port ?? 3080 }}</span>
      <span v-if="status?.pid" class="meta-item">{{ t("pid") }} {{ status.pid }}</span>
      <button class="link" @click="toggleLog">{{ t(logOpen ? "hideLog" : "log") }}</button>
    </div>
    <div v-if="logOpen" class="log">
      <div v-for="(line, i) in logLines" :key="i" class="log-line">{{ line }}</div>
      <div v-if="logLines.length === 0" class="log-empty">{{ t("noLog") }}</div>
    </div>
  </div>
</template>

<style scoped>
.status {
  display: flex;
  flex-direction: column;
  gap: 6px;
  padding: 2px 2px 8px;
  border-bottom: 1px solid var(--border);
}
.status-main {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
}
.dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  flex: none;
}
.dot.ok { background: var(--ok); }
.dot.busy { background: var(--warn); animation: blink 1s infinite; }
.dot.off { background: var(--dot-off); }
.dot.warn { background: var(--warn); }
.dot.err { background: var(--err); }
@keyframes blink { 50% { opacity: 0.25; } }
.name {
  font-size: 13px;
  font-weight: 600;
}
.msg {
  font-size: 12px;
  color: var(--muted);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.msg.warn { color: var(--err); }
.meta {
  display: flex;
  align-items: center;
  gap: 10px;
  color: var(--muted);
  font-size: 11px;
  padding-left: 16px;
}
.meta-item {
  font-variant-numeric: tabular-nums;
}
.link {
  background: none;
  border: none;
  color: var(--muted);
  cursor: pointer;
  font-size: 11px;
  padding: 0;
  margin-left: auto;
}
.link:hover { color: var(--text); }
.log {
  margin-top: 2px;
  background: var(--code-bg);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 8px;
  max-height: 140px;
  overflow-y: auto;
}
.log-line {
  font-family: var(--font-mono);
  font-size: 11px;
  color: var(--muted);
  white-space: pre-wrap;
  word-break: break-all;
  line-height: 1.5;
}
.log-empty { color: var(--faint); font-size: 11px; }
</style>
