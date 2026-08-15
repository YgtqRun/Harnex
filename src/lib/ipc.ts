import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type DshStatusKind =
  | "running"
  | "starting"
  | "stopped"
  | "portBusy"
  | "error";

export interface DshStatus {
  kind: DshStatusKind;
  pid: number | null;
  port: number;
  startedAt: number | null;
  version: string | null;
  message: string | null;
}

export interface AppConfig {
  port: number;
  dshCommand: string | null;
  workDir: string | null;
  stopOnExit: boolean;
  dshVersion: string | null;
  rememberWindowState: boolean;
}

export interface TermOutput {
  winId: number;
  runId: number;
  stream: "stdout" | "stderr";
  text: string;
}

export interface TermExit {
  winId: number;
  runId: number;
  code: number | null;
  cancelled: boolean;
}

export interface DshLog {
  stream: "stdout" | "stderr";
  text: string;
}

export interface DshTheme {
  preference: "system" | "light" | "dark" | string;
}

export const api = {
  getConfig: () => invoke<AppConfig>("get_config"),
  setConfig: (cfg: AppConfig) => invoke<AppConfig>("set_config", { cfg }),
  getStatus: () => invoke<DshStatus>("get_dsh_status"),
  start: () => invoke<DshStatus>("dsh_start"),
  stop: () => invoke<DshStatus>("dsh_stop"),
  restart: () => invoke<DshStatus>("dsh_restart"),
  getDshLog: () => invoke<string[]>("get_dsh_log"),
  termRun: (winId: number, command: string) =>
    invoke<number>("term_run", { winId, command }),
  termCancel: (winId: number) => invoke<void>("term_cancel", { winId }),
  openNativeCmd: (command?: string) =>
    invoke<void>("open_native_cmd", { command: command ?? null }),
  getWorkDir: () => invoke<string>("get_work_dir"),
  setWorkDir: (path: string) => invoke<AppConfig>("set_work_dir", { path }),
  newWindow: () => invoke<number>("new_window"),
};

export function onStatus(cb: (s: DshStatus) => void): Promise<UnlistenFn> {
  return listen<DshStatus>("dsh-status", (e) => cb(e.payload));
}

export function onConfigChanged(cb: (c: AppConfig) => void): Promise<UnlistenFn> {
  return listen<AppConfig>("config-changed", (e) => cb(e.payload));
}

export function onTermOutput(cb: (t: TermOutput) => void): Promise<UnlistenFn> {
  return listen<TermOutput>("term-output", (e) => cb(e.payload));
}

export function onTermExit(cb: (t: TermExit) => void): Promise<UnlistenFn> {
  return listen<TermExit>("term-exit", (e) => cb(e.payload));
}

export function onDshLog(cb: (l: DshLog) => void): Promise<UnlistenFn> {
  return listen<DshLog>("dsh-log", (e) => cb(e.payload));
}

export function onDshTheme(cb: (t: DshTheme) => void): Promise<UnlistenFn> {
  return listen<DshTheme>("dsh-theme", (e) => cb(e.payload));
}
