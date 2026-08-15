<script setup lang="ts">
import { reactive, ref, watch } from "vue";
import { api, type AppConfig } from "../lib/ipc";

const props = defineProps<{ config: AppConfig | null }>();
const emit = defineEmits<{ saved: [] }>();

const open = ref(false);
const saving = ref(false);
const form = reactive({
  port: 3080,
  dshCommand: "",
  workDir: "",
  stopOnExit: false,
  dshVersion: "",
  rememberWindowState: true,
});

watch(
  () => props.config,
  (c) => {
    if (!c) return;
    form.port = c.port;
    form.dshCommand = c.dshCommand ?? "";
    form.workDir = c.workDir ?? "";
    form.stopOnExit = c.stopOnExit;
    form.dshVersion = c.dshVersion ?? "";
    form.rememberWindowState = c.rememberWindowState;
  },
  { immediate: true },
);

async function pickDir() {
  const { open } = await import("@tauri-apps/plugin-dialog");
  const dir = await open({ directory: true, multiple: false, title: "选择工作目录" });
  if (typeof dir === "string") form.workDir = dir;
}

async function save() {
  saving.value = true;
  try {
    await api.setConfig({
      port: Math.max(1, Math.min(65535, Math.round(form.port) || 3080)),
      dshCommand: form.dshCommand.trim() || null,
      workDir: form.workDir.trim() || null,
      stopOnExit: form.stopOnExit,
      dshVersion: form.dshVersion.trim() || null,
      rememberWindowState: form.rememberWindowState,
    });
    emit("saved");
  } catch (e) {
    alert(String(e));
  } finally {
    saving.value = false;
  }
}
</script>

<template>
  <div class="settings">
    <button class="toggle" @click="open = !open">
      <span>{{ open ? "收起设置" : "设置" }}</span>
      <span class="chevron" :class="{ open }">▾</span>
    </button>
    <div v-if="open" class="form">
      <label class="field">
        <span>DSH 端口</span>
        <input v-model.number="form.port" type="number" min="1" max="65535" />
      </label>
      <label class="field">
        <span>命令覆盖（留空自动探测）</span>
        <input v-model="form.dshCommand" placeholder="如 dsh web --port 3080" />
      </label>
      <label class="field">
        <span>共享工作目录</span>
        <div class="dir-row">
          <input v-model="form.workDir" placeholder="默认用户主目录" />
          <button class="pick" @click="pickDir">选择</button>
        </div>
      </label>
      <label class="field">
        <span>DSH 版本锁定（npx 路径）</span>
        <input v-model="form.dshVersion" placeholder="如 0.1.0-rc.6" />
      </label>
      <label class="check">
        <input v-model="form.stopOnExit" type="checkbox" />
        <span>退出 Harnex 时停止 DSH</span>
      </label>
      <label class="check">
        <input v-model="form.rememberWindowState" type="checkbox" />
        <span>记住各窗口的大小和位置</span>
      </label>
      <button class="save" :disabled="saving" @click="save">
        {{ saving ? "保存中…" : "保存" }}
      </button>
    </div>
  </div>
</template>

<style scoped>
.settings {
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
.form {
  display: flex;
  flex-direction: column;
  gap: 10px;
  margin-top: 10px;
}
.field {
  display: flex;
  flex-direction: column;
  gap: 4px;
  font-size: 11px;
  color: var(--muted);
}
.field input {
  background: var(--panel-2);
  border: 1px solid var(--border);
  border-radius: 8px;
  color: var(--text);
  padding: 6px 9px;
  font-size: 12px;
  outline: none;
  width: 100%;
  transition: border-color 0.15s ease, box-shadow 0.15s ease;
}
.field input:focus {
  border-color: var(--accent);
  box-shadow: 0 0 0 3px var(--accent-ring);
}
.dir-row { display: flex; gap: 6px; }
.dir-row input { flex: 1; min-width: 0; }
.pick {
  background: var(--panel-2);
  border: 1px solid var(--border);
  border-radius: 8px;
  color: var(--text);
  cursor: pointer;
  padding: 0 10px;
  font-size: 11px;
}
.pick:hover { border-color: var(--border-strong); }
.check {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 12px;
}
.check input { accent-color: var(--accent); }
.save {
  background: var(--accent);
  border: none;
  border-radius: 8px;
  color: #fff;
  padding: 8px 0;
  cursor: pointer;
  font-weight: 500;
  font-size: 12px;
}
.save:hover:not(:disabled) { background: var(--accent-hover); }
.save:disabled { opacity: 0.5; cursor: not-allowed; }
</style>
