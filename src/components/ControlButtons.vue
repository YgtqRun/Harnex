<script setup lang="ts">
import { computed, ref } from "vue";
import { api, type DshStatus } from "../lib/ipc";
import { t } from "../lib/i18n";

const props = defineProps<{ status: DshStatus | null }>();
const emit = defineEmits<{ changed: [] }>();
const pending = ref<string | null>(null);

const kind = computed(() => props.status?.kind ?? "stopped");
const canStart = computed(() => !["running", "starting"].includes(kind.value));
const canStop = computed(() => ["running", "starting", "portBusy"].includes(kind.value));
const canRestart = computed(() => ["running", "starting", "portBusy"].includes(kind.value));

async function act(action: "start" | "stop" | "restart") {
  if (pending.value) return;
  pending.value = action;
  try {
    if (action === "start") await api.start();
    else if (action === "stop") await api.stop();
    else await api.restart();
  } catch (e) {
    alert(String(e));
  } finally {
    pending.value = null;
    emit("changed");
  }
}
</script>

<template>
  <div class="row">
    <button class="btn ghost" :disabled="!canStart || !!pending" @click="act('start')">
      {{ pending === "start" ? t("startingEllipsis") : t("start") }}
    </button>
    <button class="btn primary" :disabled="!canRestart || !!pending" @click="act('restart')">
      {{ pending === "restart" ? t("restartingEllipsis") : t("restart") }}
    </button>
    <button class="btn ghost danger" :disabled="!canStop || !!pending" @click="act('stop')">
      {{ pending === "stop" ? t("stoppingEllipsis") : t("stop") }}
    </button>
  </div>
</template>

<style scoped>
.row {
  display: grid;
  grid-template-columns: 1fr 1.2fr 1fr;
  gap: 8px;
}
.btn {
  border-radius: 8px;
  padding: 7px 0;
  font-size: 12px;
  font-weight: 500;
  cursor: pointer;
  transition: background 0.15s ease, border-color 0.15s ease, opacity 0.15s ease;
}
.btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}
.ghost {
  background: transparent;
  border: 1px solid var(--border);
  color: var(--text);
}
.ghost:hover:not(:disabled) {
  background: var(--hover-soft);
  border-color: var(--border-strong);
}
.ghost.danger { color: var(--err); }
.ghost.danger:hover:not(:disabled) {
  background: rgba(237, 53, 68, 0.08);
  border-color: var(--err);
}
.primary {
  background: var(--accent);
  border: 1px solid var(--accent);
  color: #fff;
}
.primary:hover:not(:disabled) { background: var(--accent-hover); }
</style>
