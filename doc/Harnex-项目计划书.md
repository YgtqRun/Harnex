# Harnex 项目计划书

> DeepSeek Harness（DSH）桌面套壳应用
>
> 版本：v1.0（定稿）　|　状态：待开工

---

## 1. 项目概述

### 1.1 项目名称

- **项目名**：Harnex（合成词，取自 **Har**ness + `-ex`，仿 Codex 的造词风格）
- **包名 / crate 名**：`harnex`
- **窗口标题**：`Harnex`（副标题可显示「DeepSeek Harness 桌面壳」）

### 1.2 背景与定位

DeepSeek Harness（DSH）目前只能通过浏览器访问（Web GUI 默认运行于 `http://127.0.0.1:3080`）。Harnex 的目标是把它装进一个原生桌面窗口，并补齐桌面场景下最常用的两个能力：**快速启停 DSH** 与 **随手敲命令**（避免为装插件等操作另开 cmd）。

Harnex 是一个**纯套壳工具**：

- 不内置 Node.js；
- 不内置 DeepSeek Harness；
- 不直接管理插件（DSH 已提供 `dsh plugin` 插件管理能力）；
- 只负责「套页面 + 启停 + 给个终端」。

### 1.3 核心价值

1. 用桌面应用方式打开 DSH，不必手动开浏览器、记端口。
2. 一键启动 / 重启 / 关闭 DSH，含状态反馈。
3. 内置命令行，装插件、跑 npx 命令不用另开 cmd。

---

## 2. 目标与约束

### 2.1 功能目标

| 编号 | 目标 |
|---|---|
| F1 | 窗口A 直接加载 DSH Web GUI（`http://127.0.0.1:3080`） |
| F2 | 应用启动时自动拉起 DSH（已在运行则复用，不重复拉起） |
| F3 | 窗口B 提供「启动 / 重启 / 关闭」快捷操作与运行状态展示 |
| F4 | 窗口B 提供简单命令框，可执行 npx 等命令并回显输出 |
| F5 | 窗口B 提供「打开原生 cmd」能力，命令框与原生 cmd 共享工作目录与环境 |
| F6 | 系统托盘：打开/关闭浮窗、退出应用 |
| F7 | 退出策略可配置：是否随应用退出停止 DSH |

### 2.2 硬约束

| 约束 | 说明 |
|---|---|
| 不内置 Node.js | 不随安装包分发 node/npm/npx，仅使用系统已有环境 |
| 不内置 DSH | 不打包 `@deepseek-ai/dsh`，运行时经 npx 解析 |
| 不接管插件管理 | 插件增删交给 DSH 自身（`dsh plugin`），Harnex 只提供终端入口 |
| 跨平台 | 优先 Windows，架构上兼容 macOS / Linux |

---

## 3. 技术选型

| 层 | 选择 | 理由 |
|---|---|---|
| 应用壳 | Tauri 2.x | 体积小、原生窗口、Rust 后端适合进程管理 |
| 后端 | Rust + `std::process` | 需管理长驻子进程、流式日志、按进程树杀进程 |
| 前端 | Vue 3 + TypeScript + Vite | 团队选型；浮窗 UI 轻量 |
| 状态存储 | `tauri-plugin-store`（JSON） | 存端口、命令覆盖、工作目录、退出策略等 |
| 进程管理 | `std::process::Command` + `Child` | 长驻进程、日志流、进程树控制 |
| 日志 | Tauri event 流 | 后端 stdout/stderr 实时推送前端 |

---

## 4. 已确认的 DSH 接口

调研结论（来自本机 `@deepseek-ai/dsh` 的 `lib/bin.js` 与 README）：

- 包名：`@deepseek-ai/dsh`，bin 名：`dsh`
- 启动 Web GUI：`dsh web`（`--profile web` 的别名，默认端口 3080，可用 `--port` 改）

```sh
# 启动 Web GUI
npx -y @deepseek-ai/dsh web
npx -y @deepseek-ai/dsh web --port 3080

# 插件管理（底层转发给 pnpm）
npx -y @deepseek-ai/dsh plugin --profile web add <pkg>
npx -y @deepseek-ai/dsh plugin --profile web remove <pkg>

# 排查配置（不启动服务）
npx -y @deepseek-ai/dsh web --dump-config
```

关键事实：

1. `dsh plugin` 内部转发给 **pnpm** —— 用户自行装插件时需要本机有 pnpm。
2. 插件按 profile 安装，Web 使用的 profile 为 `web`，目录位于 `$DSH_HOME/profiles/web`（默认 `$DSH_HOME=~/.dsh`）。
3. npx 首次运行会弹「Ok to proceed?」—— Harnex 一律带 `-y` 防止挂死。
4. 本机可能已有全局 `dsh`，也可能只有 npx 缓存 —— Harnex 优先用全局 `dsh`，找不到退回 `npx -y @deepseek-ai/dsh`，且允许用户配置覆盖。

---

## 5. 总体架构

```
┌──────────────────────────────────────────────────────┐
│                    Harnex（Tauri 2）                   │
│                                                        │
│  ┌──────────────────┐   ┌──────────────────────────┐  │
│  │ 窗口A：套壳        │   │ 窗口B：控制浮窗（Vue 3）     │  │
│  │ 加载 http://       │   │ 状态灯 + 启动/重启/关闭       │  │
│  │ 127.0.0.1:3080    │   │ + 命令框 + 打开原生 cmd      │  │
│  │ （无 IPC 权限）     │   │ （IPC 白名单）              │  │
│  └────────┬─────────┘   └───────────┬──────────────┘  │
│           │ http                     │ invoke/event    │
│           ▼                          ▼                 │
│  ┌───────────────────────────────────────────────────┐ │
│  │              Rust 后端                              │ │
│  │  process.rs（DSH 启停/重启/杀进程树/健康探测）        │ │
│  │  term.rs（命令框执行 / 拉起原生 cmd / 共享工作目录）    │ │
│  │  config.rs（配置读写）                              │ │
│  └──────────────────────┬────────────────────────────┘ │
│                         │ spawn（cmd /C npx ...）       │
│                         ▼                               │
│              本机 npx → @deepseek-ai/dsh web :3080       │
└──────────────────────────────────────────────────────┘
```

---

## 6. 窗口设计

### 6.1 窗口A：套壳窗口

- 形态：标准主窗口，webview 直接加载 `http://127.0.0.1:3080`。
- 权限：**不授予任何 Tauri IPC 权限**（远程页面安全收紧）。
- 行为：DSH 启动就绪后加载；重启后 `reload`；DSH 未运行时可显示「离线/启动中」占位。

### 6.2 窗口B：无边框浮动小窗

- 形态：无边框、置顶、可拖拽、不占任务栏。
- 内容：
  1. **状态灯**：运行中 / 启动中 / 已停止 / 端口被占。
  2. **快捷按钮**：启动、重启、关闭。
  3. **命令框**：输入框 + 输出区。
  4. **打开原生 cmd**：按钮，起始目录 = 命令框当前工作目录。
- 权限：仅放行控制相关白名单命令。

### 6.3 系统托盘

- 打开/关闭浮窗
- 退出应用
- （可选）快速启停入口

---

## 7. 后端模块设计

### 7.1 process.rs —— DSH 生命周期管理

| 能力 | 实现要点 |
|---|---|
| 启动 | `cmd /C npx -y @deepseek-ai/dsh web --port <port>`（Windows 下 npx 是 `.cmd`，须经 `cmd /C`） |
| 关闭 | Windows `taskkill /F /T /PID <pid>` 杀整棵进程树；Unix 用进程组 + `kill -TERM -- -pgid` |
| 重启 | 关闭 → 启动 → 探测端口就绪 → 通知窗口A reload |
| 状态 | 探测 `127.0.0.1:<port>`；已在外运行则复用，不重复拉起 |
| 持有 | `Mutex<Option<Child>>` 跟踪子进程句柄 |

### 7.2 term.rs —— 命令行能力

| 能力 | 实现要点 |
|---|---|
| 命令框执行 | `cmd /C <命令>`，stdout/stderr 流式回显（通过 Tauri event 推送） |
| 打开原生 cmd | 独立弹出 `cmd.exe` 窗口（Windows 用 `CREATE_NEW_CONSOLE` 或 `start`），起始目录 = 共享工作目录 |
| 一键送命令 | `cmd /K <命令>`，执行后保留窗口 |
| 互通 | 命令框与原生 cmd 共享同一个工作目录变量与环境变量；命令框记住最近命令 |

> 说明：Windows 下管道化的 cmd 进程无法再挂回控制台，命令框与原生 cmd 不可能是同一会话。交互式命令（如 npx/pnpm 安装）推荐「打开原生 cmd」执行；命令框负责快速一句式与查看输出。

### 7.3 config.rs —— 配置

- `port`：DSH 端口（默认 3080）
- `dshCommand`：命令覆盖（默认空 = 自动探测）
- `workDir`：命令框与原生 cmd 的共享工作目录
- `stopOnExit`：退出应用时是否停止 DSH（默认保留运行）
- `dshVersion`：可选版本锁定（如 `@deepseek-ai/dsh@0.1.0-rc.6`）

---

## 8. 前端设计（窗口B，Vue 3）

- **状态卡**：运行状态、端口、版本、启动耗时
- **控制按钮**：启动 / 重启 / 关闭
- **命令框组件**：输入框、输出区（滚动回显）、最近命令、一键送原生 cmd
- **设置**：端口、命令覆盖、工作目录、退出策略

---

## 9. 建议目录结构

```
harnex/
├─ src-tauri/
│  ├─ Cargo.toml
│  ├─ tauri.conf.json
│  ├─ capabilities/default.json      # 仅放行窗口B 的白名单命令
│  └─ src/
│     ├─ main.rs  lib.rs
│     ├─ commands.rs
│     ├─ process.rs
│     ├─ term.rs
│     └─ config.rs
└─ src/                              # 控制台浮窗前端（Vue 3 + Vite）
   ├─ main.ts
   ├─ App.vue
   ├─ components/
   │  ├─ StatusCard.vue
   │  ├─ ControlButtons.vue
   │  ├─ CommandBox.vue
   │  └─ SettingsPanel.vue
   └─ lib/
      └─ ipc.ts
```

---

## 10. 分阶段里程碑

| 阶段 | 内容 | 验收标准 |
|---|---|---|
| **M1 套壳 + 启停** | 窗口A 套页面、自动拉起、浮动窗按钮、托盘、退出策略 | 双击图标即见 DSH；托盘/浮窗可停止且进程干净退出 |
| **M2 命令框 + 原生 cmd** | 命令框执行、打开原生 cmd、共享目录、一键送命令 | 能直接在命令框跑 `npx` 装插件；点按钮弹出 cmd 且目录一致 |
| **M3 打包** | NSIS/MSI（Windows）+ 图标 + 可选自动更新 | 安装包不含 node/DSH；未预装 DSH 的机器首次启动可经 npx 拉起 |

---

## 11. 风险清单

| 风险 | 对策 |
|---|---|
| DSH 仍是 RC 版（`0.1.0-rc.6`），CLI/目录结构可能变动 | 命令层与端口/命令做成可配置，留兼容层 |
| Windows 下 `npx.cmd` 不能直接 exec | 经 `cmd /C` 启动 |
| 杀 npx 会留孤儿 node 子进程 | `taskkill /F /T`（Unix 用进程组） |
| npx 首次交互提示挂死 | 固定加 `-y` |
| 端口被占但未必是 DSH | 健康探测校验响应，区分「已停止/启动中/端口被占」 |
| 应用启动时 DSH 可能已运行 | 先探测再决定，复用不重复拉起 |
| 命令框与原生 cmd 非同一会话 | 明确边界，交互式命令引导到原生 cmd |
| 插件安装需 pnpm（隐藏依赖） | 命令框/引导中提示，必要时提示用 corepack 启用 |

---

## 12. 待办（开工顺序）

1. Phase 0：验证 Windows 下 npx 拉起 DSH 与 `taskkill /T` 干净收尾。
2. M1：Tauri + Vue 骨架、窗口A、自动拉起、启停、托盘、退出策略。
3. M2：命令框 + 原生 cmd 互通。
4. M3：打包分发。

---

## 13. 决策记录

| 项 | 结论 |
|---|---|
| 项目名 | Harnex |
| 前端 | Vue 3 + TypeScript |
| 窗口B 形态 | 无边框浮动小窗 |
| 启动策略 | 应用启动自动拉起 DSH |
| 命令行 | 简单命令框 + 一键拉起原生 cmd，共享目录与环境 |
| 插件管理 | 不内置，交给 DSH（`dsh plugin`），Harnex 只提供终端入口 |
| 是否内置 node/DSH | 均不内置，仅调用本机环境 |

---

*文档结束。待用户确认「开工」后进入实施。*
