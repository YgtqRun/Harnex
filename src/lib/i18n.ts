import { ref } from "vue";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type Locale = "zh" | "en";

const messages: Record<Locale, Record<string, string>> = {
  zh: {
    newWindow: "新建窗口",
    console: "控制台",
    minimize: "最小化",
    maximize: "最大化 / 还原",
    closeWindow: "关闭窗口",
    waiting: "正在等待 DeepSeek Harness 启动…",
    panelTitle: "控制台",
    zoom: "页面缩放",
    zoomOut: "缩小",
    zoomIn: "放大",
    reset: "重置",
    running: "运行中",
    starting: "启动中",
    stopped: "已停止",
    portBusy: "端口被占",
    error: "异常",
    port: "端口",
    pid: "PID",
    log: "日志",
    hideLog: "收起日志",
    noLog: "暂无日志",
    start: "启动",
    restart: "重启",
    stop: "停止",
    startingEllipsis: "启动中…",
    restartingEllipsis: "重启中…",
    stoppingEllipsis: "关闭中…",
    cmdPlaceholder:
      "输入命令，如 npx -y @deepseek-ai/dsh plugin --profile web add <pkg>",
    run: "运行",
    runningEllipsis: "执行中…",
    stopCmd: "停止",
    nativeCmd: "原生 cmd",
    exitCode: "退出码",
    clear: "清空",
    cancelled: "（已取消）",
    settings: "设置",
    collapseSettings: "收起设置",
    dshPort: "DSH 端口",
    cmdOverride: "命令覆盖（留空自动探测）",
    workDir: "共享工作目录",
    pick: "选择",
    dshVersion: "DSH 版本锁定（npx 路径）",
    stopOnExit: "退出 Harnex 时停止 DSH",
    rememberWindow: "记住各窗口的大小和位置",
    save: "保存",
    saving: "保存中…",
    about: "关于",
    harnexVersion: "Harnex 版本",
    github: "GitHub",
    githubOpen: "打开",
  },
  en: {
    newWindow: "New Window",
    console: "Console",
    minimize: "Minimize",
    maximize: "Maximize / Restore",
    closeWindow: "Close Window",
    waiting: "Waiting for DeepSeek Harness to start…",
    panelTitle: "Console",
    zoom: "Page zoom",
    zoomOut: "Zoom out",
    zoomIn: "Zoom in",
    reset: "Reset",
    running: "Running",
    starting: "Starting",
    stopped: "Stopped",
    portBusy: "Port busy",
    error: "Error",
    port: "Port",
    pid: "PID",
    log: "Log",
    hideLog: "Hide log",
    noLog: "No log",
    start: "Start",
    restart: "Restart",
    stop: "Stop",
    startingEllipsis: "Starting…",
    restartingEllipsis: "Restarting…",
    stoppingEllipsis: "Stopping…",
    cmdPlaceholder:
      "Type a command, e.g. npx -y @deepseek-ai/dsh plugin --profile web add <pkg>",
    run: "Run",
    runningEllipsis: "Running…",
    stopCmd: "Stop",
    nativeCmd: "Native cmd",
    exitCode: "Exit code",
    clear: "Clear",
    cancelled: "(cancelled)",
    settings: "Settings",
    collapseSettings: "Collapse settings",
    dshPort: "DSH port",
    cmdOverride: "Command override (auto-detect when empty)",
    workDir: "Shared working directory",
    pick: "Pick",
    dshVersion: "DSH version pin (npx path)",
    stopOnExit: "Stop DSH when exiting Harnex",
    rememberWindow: "Remember window size & position",
    save: "Save",
    saving: "Saving…",
    about: "About",
    harnexVersion: "Harnex version",
    github: "GitHub",
    githubOpen: "Open",
  },
};

function detectLocale(): Locale {
  const lang = (navigator.language || "zh").toLowerCase();
  return lang.startsWith("en") ? "en" : "zh";
}

export const locale = ref<Locale>(detectLocale());

export function applyLocale(pref: string) {
  locale.value = pref === "en" ? "en" : pref === "zh" ? "zh" : detectLocale();
  document.documentElement.lang = locale.value === "zh" ? "zh-CN" : "en";
}

export function t(key: string): string {
  return messages[locale.value][key] ?? messages.zh[key] ?? key;
}

export function formatUptime(secs: number): string {
  const m = Math.floor(secs / 60);
  const s = secs % 60;
  return locale.value === "zh" ? `${m}分${s}秒` : `${m}m ${s}s`;
}

export function onDshLocale(cb: (pref: string) => void): Promise<UnlistenFn> {
  return listen<{ preference: string }>("dsh-locale", (e) =>
    cb(e.payload.preference),
  );
}
