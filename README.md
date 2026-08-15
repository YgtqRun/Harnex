# <img src="src-tauri/icons/harnex-1024.png" width="48" height="48" alt="Harnex" /> Harnex

> 把 DeepSeek Harness 装进桌面，打开就用。

![Tauri](https://img.shields.io/badge/Tauri-2.x-24c8db?style=flat-square)
![Vue](https://img.shields.io/badge/Vue-3-42b883?style=flat-square)
![Rust](https://img.shields.io/badge/Rust-stable-dea584?style=flat-square)
![Platform](https://img.shields.io/badge/Platform-Windows-0078d6?style=flat-square)
![License](https://img.shields.io/badge/License-MIT-yellow?style=flat-square)

中文 | [English](README.en.md)

## Harnex 是什么？

Harnex 是 [DeepSeek Harness](https://github.com/deepseek-ai)（DSH）的桌面客户端。DSH 原本只能开浏览器访问，Harnex 把它变成真正的桌面应用——不用记端口、不用切窗口，双击图标就能用，还能一键启停、随手敲命令。

![Harnex 主窗口](doc/screenshots/screenshot-main.png)

## 核心特性

- **一体化桌面外壳** —— DSH 页面直接内嵌窗口，顶栏与页面同底色一体，跟随 DSH 深浅主题
- **多开窗口** —— 想开几个开几个，新窗口继承当前尺寸，全部连同一个 DSH
- **一键启停** —— 启动 / 重启 / 停止，状态实时可见，带启动日志；应用启动时自动拉起，已在运行则复用
- **控制台抽屉** —— 状态、启停、页面缩放、命令框、设置、关于，收在一个干净的右侧抽屉里
- **内置命令框** —— 输出流式回显，装插件、跑 npx 随手就来；交互式命令一键转到原生 cmd
- **页面缩放** —— 50%–200%，和浏览器 Ctrl± 一个手感，按窗口独立记忆
- **主题与语言同步** —— 在 DSH 里改配色、切语言，Harnex 外壳一秒跟上
- **窗口记忆** —— 每个窗口的位置大小各自记住，最大化状态不会被误存
- **托盘常驻** —— 关掉所有窗口也不退出，托盘直接显示 DSH 运行状态

## 快速开始

环境只需要本机有 Node.js 18+（Harnex 不自带 Node，也不打包 DSH）。

```sh
npm install
npm run tauri dev     # 开发模式
npm run tauri build   # 打包安装包
```

首次启动会自动通过 npx 拉取 DSH；已经装过全局 `dsh` 的话会优先用它。

## 小贴士

- 安装插件需要本机有 pnpm（DSH 的插件管理底层用的是 pnpm）
- 交互式命令（比如 pnpm 安装）建议放到「原生 cmd」里跑
- 全部窗口关掉后应用驻留托盘，退出请在托盘菜单里选「退出 Harnex」

## 反馈与支持

- 项目主页：[github.com/YgtqRun/Harnex](https://github.com/YgtqRun/Harnex)
- 遇到问题或想提需求：去 [GitHub Issues](https://github.com/YgtqRun/Harnex/issues) 告诉我们

如果 Harnex 对你有帮助，别忘了点个 Star。

## 许可证

[MIT](LICENSE)

> Harnex 是独立第三方项目，与 DeepSeek 官方无关。
