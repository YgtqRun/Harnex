<script setup lang="ts">
import { nextTick, onMounted, onUnmounted, ref } from "vue";
import { api, onTermExit, onTermOutput } from "../lib/ipc";

defineProps<{ workDir: string }>();

interface OutLine {
  stream: "out" | "err" | "info";
  text: string;
}

const input = ref("");
const output = ref<OutLine[]>([]);
const outEl = ref<HTMLElement | null>(null);
const running = ref(false);
const lastCode = ref<number | null>(null);
const recent = ref<string[]>(
  JSON.parse(localStorage.getItem("harnex.recent") ?? "[]"),
);
const historyIdx = ref(-1);
const hintsOpen = ref(false);

function push(stream: OutLine["stream"], text: string) {
  output.value.push({ stream, text });
  if (output.value.length > 2000) {
    output.value.splice(0, output.value.length - 2000);
  }
  void nextTick(() => {
    if (outEl.value) outEl.value.scrollTop = outEl.value.scrollHeight;
  });
}

async function run() {
  const cmd = input.value.trim();
  if (!cmd || running.value) return;
  running.value = true;
  lastCode.value = null;
  push("info", `> ${cmd}`);
  try {
    await api.termRun(cmd);
    const list = recent.value.filter((c) => c !== cmd);
    list.unshift(cmd);
    recent.value = list.slice(0, 20);
    localStorage.setItem("harnex.recent", JSON.stringify(recent.value));
  } catch (e) {
    push("err", String(e));
    running.value = false;
  }
}

function stop() {
  api.termCancel().catch(() => {});
}

function openNative() {
  const cmd = input.value.trim();
  api.openNativeCmd(cmd || undefined).catch((e) => alert(String(e)));
}

function pick(cmd: string) {
  input.value = cmd;
  hintsOpen.value = false;
}

function blurSoon() {
  window.setTimeout(() => {
    hintsOpen.value = false;
  }, 150);
}

function onKey(e: KeyboardEvent) {
  if (e.key === "ArrowUp") {
    e.preventDefault();
    if (recent.value.length === 0) return;
    historyIdx.value = Math.min(historyIdx.value + 1, recent.value.length - 1);
    input.value = recent.value[historyIdx.value];
  } else if (e.key === "ArrowDown") {
    e.preventDefault();
    if (historyIdx.value < 0) return;
    historyIdx.value -= 1;
    input.value =
      historyIdx.value >= 0 ? recent.value[historyIdx.value] : "";
  }
}

function clearOut() {
  output.value = [];
  lastCode.value = null;
}

let unlisteners: Array<() => void> = [];
onMounted(async () => {
  unlisteners.push(
    await onTermOutput((t) =>
      push(t.stream === "stderr" ? "err" : "out", t.text),
    ),
  );
  unlisteners.push(
    await onTermExit((t) => {
      running.value = false;
      lastCode.value = t.cancelled ? 0 : (t.code ?? 0);
      push("info", t.cancelled ? "（已取消）" : `[退出码 ${t.code ?? 0}]`);
    }),
  );
});
onUnmounted(() => unlisteners.forEach((u) => u()));
</script>

<template>
  <section class="box">
    <div class="bar">
      <span class="title">命令框</span>
      <span class="hint">交互式命令请用原生 cmd</span>
    </div>
    <div class="input-row">
      <input
        v-model="input"
        class="cmd-input"
        placeholder="输入命令，如 npx -y @deepseek-ai/dsh plugin --profile web add <pkg>"
        :disabled="running"
        @keydown="onKey"
        @keydown.enter.prevent="run"
        @focus="hintsOpen = true"
        @blur="blurSoon"
      />
      <button class="mini run" :disabled="running" @click="run">运行</button>
      <button class="mini stop" :disabled="!running" @click="stop">停止</button>
    </div>
    <div v-if="hintsOpen && recent.length" class="hints">
      <div
        v-for="(c, i) in recent.slice(0, 6)"
        :key="i"
        class="hint-item"
        @mousedown.prevent="pick(c)"
      >
        {{ c }}
      </div>
    </div>
    <pre ref="outEl" class="output">
<template v-for="(line, i) in output" :key="i"><span :class="line.stream">{{ line.text }}</span>
</template></pre>
    <div class="foot">
      <span v-if="lastCode !== null" class="code">退出码 {{ lastCode }}</span>
      <span v-else class="code muted">—</span>
      <div class="foot-actions">
        <button class="link" @click="openNative">在原生 cmd 中打开</button>
        <button class="link" @click="clearOut">清空</button>
      </div>
    </div>
  </section>
</template>

<style scoped>
.box {
  background: var(--panel);
  border: 1px solid var(--border);
  border-radius: 10px;
  padding: 12px 14px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.bar {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
.title { font-weight: 600; }
.hint { color: var(--muted); font-size: 11px; }
.input-row { display: flex; gap: 6px; }
.cmd-input {
  flex: 1;
  min-width: 0;
  background: var(--panel-2);
  border: 1px solid var(--border);
  border-radius: 8px;
  color: var(--text);
  padding: 7px 9px;
  font-size: 12px;
  outline: none;
}
.cmd-input:focus { border-color: var(--accent); }
.mini {
  border: none;
  border-radius: 8px;
  padding: 0 12px;
  font-size: 12px;
  cursor: pointer;
  color: #fff;
}
.mini:disabled { opacity: 0.4; cursor: not-allowed; }
.run { background: var(--accent); }
.stop { background: #b23a38; }
.hints {
  background: var(--panel-2);
  border: 1px solid var(--border);
  border-radius: 8px;
  max-height: 140px;
  overflow-y: auto;
}
.hint-item {
  padding: 6px 9px;
  font-family: Consolas, monospace;
  font-size: 12px;
  cursor: pointer;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.hint-item:hover { background: #2a3040; }
.output {
  margin: 0;
  background: #0d0f14;
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 8px;
  height: 170px;
  overflow-y: auto;
  font-family: Consolas, monospace;
  font-size: 12px;
  line-height: 1.5;
  white-space: pre-wrap;
  word-break: break-all;
}
.output :deep(.out) { color: #c8d3e5; }
.output :deep(.err) { color: #ff8a86; }
.output :deep(.info) { color: #8b92a5; }
.foot {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
.code { font-family: Consolas, monospace; font-size: 11px; }
.muted { color: var(--muted); }
.foot-actions { display: flex; gap: 10px; }
.link {
  background: none;
  border: none;
  color: var(--accent);
  cursor: pointer;
  font-size: 12px;
  padding: 0;
}
</style>
