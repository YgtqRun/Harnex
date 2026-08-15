<script setup lang="ts">
import { computed, ref } from "vue";
import { api, type DshStatus } from "../lib/ipc";

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
  <section class="btns">
    <button class="btn start" :disabled="!canStart || !!pending" @click="act('start')">
      {{ pending === "start" ? "启动中…" : "启动" }}
    </button>
    <button class="btn restart" :disabled="!canRestart || !!pending" @click="act('restart')">
      {{ pending === "restart" ? "重启中…" : "重启" }}
    </button>
    <button class="btn stop" :disabled="!canStop || !!pending" @click="act('stop')">
      {{ pending === "stop" ? "关闭中…" : "关闭" }}
    </button>
  </section>
</template>

<style scoped>
.btns {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 8px;
}
.btn {
  border: none;
  border-radius: 8px;
  padding: 9px 0;
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
  color: #fff;
  transition: filter 0.15s;
}
.btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}
.btn:not(:disabled):hover { filter: brightness(1.12); }
.start { background: #1f8a5d; }
.restart { background: #9a6b12; }
.stop { background: #b23a38; }
</style>
