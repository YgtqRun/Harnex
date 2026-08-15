<script setup lang="ts">
import { computed, onUnmounted, ref, watch } from "vue";
import { api, type AppConfig, type DshStatus } from "../lib/ipc";

const props = defineProps<{
  status: DshStatus | null;
  config: AppConfig | null;
}>();

const LABELS: Record<string, { text: string; cls: string }> = {
  running: { text: "运行中", cls: "ok" },
  starting: { text: "启动中", cls: "busy" },
  stopped: { text: "已停止", cls: "off" },
  portBusy: { text: "端口被占", cls: "warn" },
  error: { text: "异常", cls: "err" },
};

const label = computed(
  () => LABELS[props.status?.kind ?? "stopped"] ?? LABELS.stopped,
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
        uptime.value = `${Math.floor(secs / 60)}分${secs % 60}秒`;
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
  <section class="card">
    <div class="row head">
      <span class="dot" :class="label.cls"></span>
      <span class="state">{{ label.text }}</span>
      <span v-if="status?.message" class="msg">{{ status.message }}</span>
    </div>
    <div class="stats">
      <div class="stat">
        <span class="k">端口</span>
        <span class="v">{{ status?.port ?? config?.port ?? 3080 }}</span>
      </div>
      <div class="stat">
        <span class="k">PID</span>
        <span class="v">{{ status?.pid ?? "—" }}</span>
      </div>
      <div class="stat">
        <span class="k">版本</span>
        <span class="v">{{ status?.version ?? "—" }}</span>
      </div>
      <div class="stat">
        <span class="k">已运行</span>
        <span class="v">{{ uptime || "—" }}</span>
      </div>
    </div>
    <button class="link-btn" @click="toggleLog">
      {{ logOpen ? "收起启动日志" : "查看启动日志" }}
    </button>
    <div v-if="logOpen" class="log">
      <div v-for="(line, i) in logLines" :key="i" class="log-line">{{ line }}</div>
      <div v-if="logLines.length === 0" class="log-empty">暂无日志</div>
    </div>
  </section>
</template>

<style scoped>
.card {
  background: var(--panel);
  border: 1px solid var(--border);
  border-radius: 10px;
  padding: 12px 14px;
}
.row {
  display: flex;
  align-items: center;
  gap: 8px;
}
.dot {
  width: 10px;
  height: 10px;
  border-radius: 50%;
  flex: none;
}
.dot.ok { background: var(--ok); box-shadow: 0 0 8px var(--ok); }
.dot.busy { background: var(--warn); animation: blink 1s infinite; }
.dot.off { background: #555d6e; }
.dot.warn { background: var(--warn); }
.dot.err { background: var(--err); }
@keyframes blink { 50% { opacity: 0.25; } }
.state { font-weight: 600; }
.msg { color: var(--muted); font-size: 12px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.stats {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 8px;
  margin-top: 12px;
}
.stat {
  background: var(--panel-2);
  border-radius: 8px;
  padding: 8px;
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
}
.k { color: var(--muted); font-size: 11px; }
.v { font-family: Consolas, monospace; font-size: 12px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.link-btn {
  background: none;
  border: none;
  color: var(--accent);
  cursor: pointer;
  padding: 6px 0 0;
  font-size: 12px;
}
.log {
  margin-top: 8px;
  background: #0d0f14;
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 8px;
  max-height: 130px;
  overflow-y: auto;
}
.log-line {
  font-family: Consolas, monospace;
  font-size: 11px;
  color: #aab2c2;
  white-space: pre-wrap;
  word-break: break-all;
  line-height: 1.45;
}
.log-empty { color: var(--muted); font-size: 12px; }
</style>
