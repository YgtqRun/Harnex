//! 命令框：一次性命令执行（流式回显）与原生 cmd 拉起。
//! 每个窗口（winId）有独立的命令会话，事件带 winId 供前端过滤。

use crate::config;
use serde::Serialize;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read};
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

#[cfg(not(windows))]
use std::os::unix::process::CommandExt;
#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[derive(Default)]
pub struct TermState {
    pub child: Option<Child>,
    pub run_id: u64,
}

pub type TermRegistry = Mutex<HashMap<u32, TermState>>;

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TermOutput {
    pub win_id: u32,
    pub run_id: u64,
    pub stream: String,
    pub text: String,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TermExit {
    pub win_id: u32,
    pub run_id: u64,
    pub code: Option<i32>,
    pub cancelled: bool,
}

/// 在指定窗口执行一条命令，stdout/stderr 通过 `term-output` 事件流式推送。
pub fn run_command(app: &AppHandle, win_id: u32, command: String) -> Result<u64, String> {
    let cmd = command.trim();
    if cmd.is_empty() {
        return Err("命令为空".to_string());
    }
    let wd = config::work_dir(app);

    let state = app.state::<TermRegistry>();
    let mut map = state.lock().unwrap();
    let entry = map.entry(win_id).or_default();
    if entry.child.is_some() {
        return Err("已有命令正在运行，请等待完成或先停止".to_string());
    }

    #[cfg(windows)]
    let mut c = {
        let mut c = Command::new("cmd");
        c.arg("/D")
            .arg("/C")
            .arg(format!("chcp 65001 >nul & {cmd}"));
        // CREATE_NO_WINDOW：命令框在后台执行，不弹窗口
        c.creation_flags(0x0800_0000);
        c
    };
    #[cfg(not(windows))]
    let mut c = {
        let mut c = Command::new("sh");
        c.arg("-c").arg(cmd);
        c.process_group(0);
        c
    };
    c.stdout(Stdio::piped()).stderr(Stdio::piped());
    if wd.exists() {
        c.current_dir(&wd);
    }

    let mut child = c.spawn().map_err(|e| format!("执行失败: {e}"))?;
    entry.run_id += 1;
    let run_id = entry.run_id;
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    entry.child = Some(child);
    drop(map);

    if let Some(o) = stdout {
        spawn_term_reader(app.clone(), o, "stdout", win_id, run_id);
    }
    if let Some(e) = stderr {
        spawn_term_reader(app.clone(), e, "stderr", win_id, run_id);
    }
    spawn_term_watcher(app.clone(), win_id, run_id);
    Ok(run_id)
}

fn spawn_term_reader(
    app: AppHandle,
    stream: impl Read + Send + 'static,
    stream_name: &'static str,
    win_id: u32,
    run_id: u64,
) {
    std::thread::spawn(move || {
        let reader = BufReader::new(stream);
        for line in reader.lines() {
            let line = match line {
                Ok(l) => l,
                Err(_) => break,
            };
            if line.trim().is_empty() {
                continue;
            }
            let _ = app.emit(
                "term-output",
                TermOutput {
                    win_id,
                    run_id,
                    stream: stream_name.to_string(),
                    text: line,
                },
            );
        }
    });
}

fn spawn_term_watcher(app: AppHandle, win_id: u32, run_id: u64) {
    std::thread::spawn(move || {
        let code = loop {
            {
                let state = app.state::<TermRegistry>();
                let mut map = state.lock().unwrap();
                match map.get_mut(&win_id).and_then(|st| st.child.as_mut()) {
                    Some(child) => match child.try_wait() {
                        Ok(Some(status)) => {
                            if let Some(st) = map.get_mut(&win_id) {
                                st.child = None;
                            }
                            break status.code();
                        }
                        Ok(None) => {}
                        Err(_) => {
                            if let Some(st) = map.get_mut(&win_id) {
                                st.child = None;
                            }
                            break None;
                        }
                    },
                    None => break None,
                }
            }
            std::thread::sleep(Duration::from_millis(80));
        };
        let _ = app.emit(
            "term-exit",
            TermExit {
                win_id,
                run_id,
                code,
                cancelled: code.is_none(),
            },
        );
    });
}

#[cfg(windows)]
fn kill_tree(pid: u32) {
    let mut cmd = Command::new("taskkill");
    cmd.args(["/F", "/T", "/PID", &pid.to_string()]);
    cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    let _ = cmd.status();
}

#[cfg(not(windows))]
fn kill_tree(pid: u32) {
    let _ = Command::new("kill")
        .arg("-TERM")
        .arg(format!("-{pid}"))
        .status();
}

pub fn cancel_command(app: &AppHandle, win_id: u32) -> Result<(), String> {
    let pid = {
        let state = app.state::<TermRegistry>();
        let map = state.lock().unwrap();
        map.get(&win_id).and_then(|st| st.child.as_ref()).map(|c| c.id())
    };
    if let Some(pid) = pid {
        kill_tree(pid);
    }
    Ok(())
}

/// 弹出原生 cmd（新控制台窗口），起始目录 = 共享工作目录；
/// 传入命令时用 `/K` 执行并保留窗口。
pub fn open_native_cmd(app: &AppHandle, command: Option<String>) -> Result<(), String> {
    let wd = config::work_dir(app);
    let cmdline = command
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    #[cfg(windows)]
    let mut c = {
        let mut c = Command::new("cmd");
        match &cmdline {
            Some(line) => {
                c.arg("/K").arg(line);
            }
            None => {
                c.arg("/K");
            }
        }
        // CREATE_NEW_CONSOLE：独立可见的 cmd 窗口
        c.creation_flags(0x0000_0010);
        c
    };
    #[cfg(not(windows))]
    let mut c = {
        let mut terminal: Option<Command> = None;
        for t in [
            "x-terminal-emulator",
            "gnome-terminal",
            "konsole",
            "xfce4-terminal",
            "xterm",
        ] {
            let found = Command::new("sh")
                .arg("-c")
                .arg(format!("command -v {t}"))
                .output();
            if found.map(|o| o.status.success()).unwrap_or(false) {
                terminal = Some(Command::new(t));
                break;
            }
        }
        terminal.ok_or_else(|| "未找到可用的终端模拟器".to_string())?
    };

    if wd.exists() {
        c.current_dir(&wd);
    }
    let _child = c.spawn().map_err(|e| format!("打开原生终端失败: {e}"))?;
    Ok(())
}
