<script setup lang="ts">
import { nextTick, onMounted, onUnmounted, ref } from "vue";
import { api, onTermExit, onTermOutput } from "../lib/ipc";

const props = defineProps<{ winId: number; workDir: string }>();

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
    await api.termRun(props.winId, cmd);
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
  api.termCancel(props.winId).catch(() => {});
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
    input.value = historyIdx.value >= 0 ? recent.value[historyIdx.value] : "";
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
      t.winId === props.winId
        ? push(t.stream === "stderr" ? "err" : "out", t.text)
        : undefined,
    ),
  );
  unlisteners.push(
    await onTermExit((t) => {
      if (t.winId !== props.winId) return;
      running.value = false;
      lastCode.value = t.cancelled ? 0 : (t.code ?? 0);
      push("info", t.cancelled ? "（已取消）" : `[退出码 ${t.code ?? 0}]`);
    }),
  );
});
onUnmounted(() => unlisteners.forEach((u) => u()));
</script>

<template>
  <div class="term">
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
      <button class="run" :disabled="running" @click="run">
        {{ running ? "执行中…" : "运行" }}
      </button>
      <button v-if="running" class="stop" @click="stop">停止</button>
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
      <button class="link" @click="openNative">原生 cmd</button>
      <span class="code">{{ lastCode !== null ? `退出码 ${lastCode}` : "—" }}</span>
      <button class="link" @click="clearOut">清空</button>
    </div>
  </div>
</template>

<style scoped>
.term {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.input-row {
  display: flex;
  gap: 6px;
}
.cmd-input {
  flex: 1;
  min-width: 0;
  background: var(--panel-2);
  border: 1px solid var(--border);
  border-radius: 8px;
  color: var(--text);
  padding: 7px 10px;
  font-size: 12px;
  font-family: var(--font-mono);
  outline: none;
  transition: border-color 0.15s ease, box-shadow 0.15s ease;
}
.cmd-input:focus {
  border-color: var(--accent);
  box-shadow: 0 0 0 3px var(--accent-ring);
}
.run,
.stop {
  border: none;
  border-radius: 8px;
  padding: 0 14px;
  font-size: 12px;
  font-weight: 500;
  cursor: pointer;
  color: #fff;
}
.run { background: var(--accent); }
.run:hover:not(:disabled) { background: var(--accent-hover); }
.run:disabled { opacity: 0.5; cursor: not-allowed; }
.stop { background: var(--err); }
.hints {
  background: var(--panel-2);
  border: 1px solid var(--border);
  border-radius: 8px;
  max-height: 140px;
  overflow-y: auto;
}
.hint-item {
  padding: 6px 10px;
  font-family: var(--font-mono);
  font-size: 12px;
  cursor: pointer;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.hint-item:hover { background: var(--hover-soft); }
.output {
  margin: 0;
  background: var(--code-bg);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 8px 10px;
  height: 170px;
  overflow-y: auto;
  font-family: var(--font-mono);
  font-size: 12px;
  line-height: 1.55;
  white-space: pre-wrap;
  word-break: break-all;
}
.output :deep(.out) { color: var(--text); }
.output :deep(.err) { color: var(--err-hover); }
.output :deep(.info) { color: var(--faint); }
.foot {
  display: flex;
  align-items: center;
  gap: 12px;
  color: var(--faint);
  font-size: 11px;
}
.code {
  font-family: var(--font-mono);
  font-variant-numeric: tabular-nums;
}
.link {
  background: none;
  border: none;
  color: var(--muted);
  cursor: pointer;
  font-size: 11px;
  padding: 0;
}
.link:first-child { margin-right: auto; }
.link:hover { color: var(--text); }
</style>
