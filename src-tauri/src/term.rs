//! 命令框：一次性命令执行（流式回显）与原生 cmd 拉起。

use crate::config;
use serde::Serialize;
use std::io::{BufRead, BufReader, Read};
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

#[cfg(not(windows))]
use std::os::unix::process::CommandExt;
#[cfg(windows)]
use std::os::windows::process::CommandExt;

pub struct TermState {
    pub child: Option<Child>,
    pub run_id: u64,
}

impl Default for TermState {
    fn default() -> Self {
        Self {
            child: None,
            run_id: 0,
        }
    }
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TermOutput {
    pub run_id: u64,
    pub stream: String,
    pub text: String,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TermExit {
    pub run_id: u64,
    pub code: Option<i32>,
    pub cancelled: bool,
}

/// 执行一条命令，stdout/stderr 通过 `term-output` 事件流式推送。
pub fn run_command(app: &AppHandle, command: String) -> Result<u64, String> {
    let cmd = command.trim();
    if cmd.is_empty() {
        return Err("命令为空".to_string());
    }
    let wd = config::work_dir(app);

    let state = app.state::<Mutex<TermState>>();
    let mut state = state.lock().unwrap();
    if state.child.is_some() {
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
    let run_id = state.run_id;
    state.run_id += 1;
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    state.child = Some(child);
    drop(state);

    if let Some(o) = stdout {
        spawn_term_reader(app.clone(), o, "stdout", run_id);
    }
    if let Some(e) = stderr {
        spawn_term_reader(app.clone(), e, "stderr", run_id);
    }
    spawn_term_watcher(app.clone(), run_id);
    Ok(run_id)
}

fn spawn_term_reader(
    app: AppHandle,
    stream: impl Read + Send + 'static,
    stream_name: &'static str,
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
                    run_id,
                    stream: stream_name.to_string(),
                    text: line,
                },
            );
        }
    });
}

fn spawn_term_watcher(app: AppHandle, run_id: u64) {
    std::thread::spawn(move || {
        let code = loop {
            {
                let state = app.state::<Mutex<TermState>>();
                let mut state = state.lock().unwrap();
                match state.child.as_mut() {
                    Some(child) => match child.try_wait() {
                        Ok(Some(status)) => {
                            state.child = None;
                            break status.code();
                        }
                        Ok(None) => {}
                        Err(_) => {
                            state.child = None;
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
                run_id,
                code,
                cancelled: code.is_none(),
            },
        );
    });
}

#[cfg(windows)]
fn kill_tree(pid: u32) {
    let _ = Command::new("taskkill")
        .args(["/F", "/T", "/PID", &pid.to_string()])
        .status();
}

#[cfg(not(windows))]
fn kill_tree(pid: u32) {
    let _ = Command::new("kill")
        .arg("-TERM")
        .arg(format!("-{pid}"))
        .status();
}

pub fn cancel_command(app: &AppHandle) -> Result<(), String> {
    let pid = {
        let state = app.state::<Mutex<TermState>>();
        let guard = state.lock().unwrap();
        guard.child.as_ref().map(|c| c.id())
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
