<script setup lang="ts">
import type { AppConfig, DshStatus } from "../lib/ipc";
import CommandBox from "./CommandBox.vue";
import ControlButtons from "./ControlButtons.vue";
import SettingsPanel from "./SettingsPanel.vue";
import StatusCard from "./StatusCard.vue";

defineProps<{
  status: DshStatus | null;
  config: AppConfig | null;
}>();

const emit = defineEmits<{ changed: []; close: [] }>();
</script>

<template>
  <section class="panel">
    <header class="head">
      <span class="title">Harnex 控制台</span>
      <button class="close" title="收起" @click="emit('close')">
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
      <CommandBox :work-dir="config?.workDir ?? ''" />
      <SettingsPanel :config="config" @saved="emit('changed')" />
    </div>
  </section>
</template>

<style scoped>
.panel {
  position: absolute;
  top: 8px;
  right: 12px;
  width: 420px;
  max-width: calc(100% - 24px);
  max-height: calc(100% - 16px);
  display: flex;
  flex-direction: column;
  background: var(--panel);
  border: 1px solid var(--border-strong);
  border-radius: 12px;
  box-shadow: 0 16px 48px rgba(0, 0, 0, 0.35);
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
</style>
