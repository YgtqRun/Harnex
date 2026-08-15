//! DSH 生命周期管理：启动 / 停止 / 重启 / 状态探测 / 日志流。

use crate::config::{self, AppConfig};
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager};

#[cfg(not(windows))]
use std::os::unix::process::CommandExt;
#[cfg(windows)]
use std::os::windows::process::CommandExt;

pub const STATUS_STOPPED: &str = "stopped";
pub const STATUS_STARTING: &str = "starting";
pub const STATUS_RUNNING: &str = "running";
pub const STATUS_PORT_BUSY: &str = "portBusy";
pub const STATUS_ERROR: &str = "error";

/// DSH 页面 HTML 中的特征标记，用于区分 DSH 与其他占用端口的程序。
const DSH_MARKER: &str = "__DSH_BOOT__";
const LOG_CAP: usize = 300;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DshStatus {
    pub kind: String,
    pub pid: Option<u32>,
    pub port: u16,
    pub started_at: Option<u64>,
    pub version: Option<String>,
    pub message: Option<String>,
}

pub struct DshState {
    pub child: Option<Child>,
    pub kind: String,
    pub pid: Option<u32>,
    pub started_at: Option<u64>,
    pub message: Option<String>,
    pub log: Vec<String>,
    pub version: Option<String>,
}

impl Default for DshState {
    fn default() -> Self {
        Self {
            child: None,
            kind: STATUS_STOPPED.to_string(),
            pid: None,
            started_at: None,
            message: None,
            log: Vec::new(),
            version: None,
        }
    }
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct DshLogLine {
    stream: String,
    text: String,
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[derive(Debug, PartialEq)]
pub enum Probe {
    Healthy,
    PortOpen,
    Closed,
    Failed(String),
}

/// 探测 `127.0.0.1:port`：能返回带 `__DSH_BOOT__` 标记的页面才算 DSH 就绪。
pub fn probe(port: u16, timeout: Duration) -> Probe {
    let addr: SocketAddr = format!("127.0.0.1:{port}")
        .parse()
        .unwrap_or_else(|_| SocketAddr::from(([127, 0, 0, 1], port)));
    let mut stream = match TcpStream::connect_timeout(&addr, timeout) {
        Ok(s) => s,
        Err(e) => match e.kind() {
            std::io::ErrorKind::ConnectionRefused | std::io::ErrorKind::TimedOut => {
                return Probe::Closed
            }
            _ => return Probe::Failed(format!("端口探测失败: {e}")),
        },
    };
    let _ = stream.set_read_timeout(Some(timeout));
    let req = format!("GET / HTTP/1.0\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n");
    if stream.write_all(req.as_bytes()).is_err() {
        return Probe::PortOpen;
    }
    let mut buf = Vec::with_capacity(4096);
    let mut tmp = [0u8; 2048];
    loop {
        match stream.read(&mut tmp) {
            Ok(0) => break,
            Ok(n) => {
                buf.extend_from_slice(&tmp[..n]);
                if buf.len() > 131_072 {
                    break;
                }
            }
            Err(_) => break,
        }
    }
    let body = String::from_utf8_lossy(&buf);
    if body.contains(DSH_MARKER) {
        Probe::Healthy
    } else {
        Probe::PortOpen
    }
}

/// 通过 netstat 找到监听端口的进程 PID（Windows）。
#[cfg(windows)]
pub fn listening_pid(port: u16) -> Option<u32> {
    let out = Command::new("netstat")
        .args(["-ano", "-p", "tcp"])
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    let needle = format!(":{port}");
    for line in text.lines() {
        let cols: Vec<&str> = line.split_whitespace().collect();
        if cols.len() >= 5
            && cols[0].eq_ignore_ascii_case("tcp")
            && cols[3] == "LISTENING"
            && cols[1].ends_with(&needle)
        {
            if let Ok(pid) = cols[4].parse() {
                return Some(pid);
            }
        }
    }
    None
}

#[cfg(not(windows))]
pub fn listening_pid(_port: u16) -> Option<u32> {
    None
}

pub fn status_snapshot(st: &DshState, port: u16) -> DshStatus {
    DshStatus {
        kind: st.kind.clone(),
        pid: st.pid,
        port,
        started_at: st.started_at,
        version: st.version.clone(),
        message: st.message.clone(),
    }
}

pub fn status_of(app: &AppHandle, port: u16) -> DshStatus {
    let state = app.state::<Mutex<DshState>>();
    let st = state.lock().unwrap();
    status_snapshot(&st, port)
}

pub fn emit_status(app: &AppHandle) {
    let port = config::load(app).port;
    let state = app.state::<Mutex<DshState>>();
    let status = {
        let st = state.lock().unwrap();
        status_snapshot(&st, port)
    };
    let _ = app.emit("dsh-status", status);
}

pub fn get_log(app: &AppHandle) -> Vec<String> {
    let state = app.state::<Mutex<DshState>>();
    let st = state.lock().unwrap();
    st.log.clone()
}

fn push_log(app: &AppHandle, line: String) {
    let state = app.state::<Mutex<DshState>>();
    let mut st = state.lock().unwrap();
    st.log.push(line);
    if st.log.len() > LOG_CAP {
        let over = st.log.len() - LOG_CAP;
        st.log.drain(..over);
    }
}

fn set_kind(app: &AppHandle, kind: &str, message: Option<String>) {
    let state = app.state::<Mutex<DshState>>();
    let mut st = state.lock().unwrap();
    st.kind = kind.to_string();
    st.message = message;
    if kind == STATUS_STOPPED {
        st.pid = None;
        st.started_at = None;
        st.version = None;
    }
}

fn child_exited(app: &AppHandle) -> bool {
    let state = app.state::<Mutex<DshState>>();
    let mut st = state.lock().unwrap();
    let exited = match st.child.as_mut() {
        Some(child) => matches!(child.try_wait(), Ok(Some(_))),
        None => false,
    };
    if exited {
        st.child = None;
    }
    exited
}

/// 刷新状态并推送：运行中 / 启动中 / 已停止 / 端口被占 / 异常。
pub fn refresh_status(app: &AppHandle) -> DshStatus {
    let port = config::load(app).port;
    let probe_result = probe(port, Duration::from_millis(1500));
    {
        let state = app.state::<Mutex<DshState>>();
        let mut st = state.lock().unwrap();
        if st.child.is_some() {
            if let Ok(Some(_)) = st.child.as_mut().unwrap().try_wait() {
                st.child = None;
                st.started_at = None;
            }
        }
        match probe_result {
            Probe::Healthy => {
                st.kind = STATUS_RUNNING.to_string();
                st.message = None;
                st.pid = listening_pid(port).or(st.pid);
            }
            Probe::PortOpen => {
                st.kind = STATUS_PORT_BUSY.to_string();
                st.message = Some(format!("端口 {port} 已被其他程序占用"));
                st.pid = listening_pid(port).or(st.pid);
                st.started_at = None;
            }
            Probe::Closed => {
                if st.child.is_none() {
                    st.kind = STATUS_STOPPED.to_string();
                    st.pid = None;
                    st.started_at = None;
                    st.message = None;
                } else {
                    st.kind = STATUS_STARTING.to_string();
                    st.message = Some("正在启动…".to_string());
                }
            }
            Probe::Failed(e) => {
                st.kind = STATUS_ERROR.to_string();
                st.message = Some(e);
            }
        }
    }
    emit_status(app);
    status_of(app, port)
}

#[cfg(windows)]
fn build_shell_command(command_line: &str) -> Command {
    let mut c = Command::new("cmd");
    c.arg("/D")
        .arg("/C")
        .arg(format!("chcp 65001 >nul & {command_line}"));
    // CREATE_NO_WINDOW：后台进程不弹黑色控制台窗口
    c.creation_flags(0x0800_0000);
    c
}

#[cfg(not(windows))]
fn build_shell_command(command_line: &str) -> Command {
    let mut c = Command::new("sh");
    c.arg("-c").arg(command_line);
    // 独立进程组，方便整组终止
    c.process_group(0);
    c
}

#[cfg(windows)]
fn global_dsh_exists() -> bool {
    Command::new("where.exe")
        .arg("dsh")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[cfg(not(windows))]
fn global_dsh_exists() -> bool {
    Command::new("sh")
        .args(["-c", "command -v dsh"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn dsh_command_line(cfg: &AppConfig) -> String {
    if let Some(over) = &cfg.dsh_command {
        let over = over.trim();
        if !over.is_empty() {
            return over.to_string();
        }
    }
    let pkg = match &cfg.dsh_version {
        Some(v) if !v.trim().is_empty() => format!("@deepseek-ai/dsh@{}", v.trim()),
        _ => "@deepseek-ai/dsh".to_string(),
    };
    if global_dsh_exists() {
        format!("dsh web --port {}", cfg.port)
    } else {
        format!("npx -y {pkg} web --port {}", cfg.port)
    }
}

fn spawn_dsh_reader(
    app: AppHandle,
    stream: impl Read + Send + 'static,
    stream_name: &'static str,
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
                "dsh-log",
                DshLogLine {
                    stream: stream_name.to_string(),
                    text: line.clone(),
                },
            );
            let state = app.state::<Mutex<DshState>>();
            let mut st = state.lock().unwrap();
            st.log.push(format!("[{stream_name}] {line}"));
            if st.log.len() > LOG_CAP {
                let over = st.log.len() - LOG_CAP;
                st.log.drain(..over);
            }
        }
    });
}

/// 启动后的后台监视：等待端口就绪 → 更新状态 → 通知窗口A 加载 DSH 页面。
fn spawn_dsh_watcher(app: AppHandle, port: u16) {
    std::thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(120);
        loop {
            match probe(port, Duration::from_millis(1200)) {
                Probe::Healthy => {
                    set_kind(&app, STATUS_RUNNING, None);
                    emit_status(&app);
                    spawn_version_fetch(&app);
                    break;
                }
                Probe::PortOpen => {
                    set_kind(
                        &app,
                        STATUS_PORT_BUSY,
                        Some(format!("端口 {port} 被其他程序占用")),
                    );
                    emit_status(&app);
                    break;
                }
                _ => {
                    if child_exited(&app) {
                        set_kind(
                            &app,
                            STATUS_STOPPED,
                            Some("进程已退出，请查看日志".to_string()),
                        );
                        emit_status(&app);
                        break;
                    }
                    if Instant::now() > deadline {
                        push_log(&app, "启动超时，正在终止…".to_string());
                        stop_dsh_inner(&app, Some("启动超时"));
                        emit_status(&app);
                        break;
                    }
                }
            }
            std::thread::sleep(Duration::from_millis(600));
        }
    });
}

/// 启动 DSH：已在运行则复用；端口被非 DSH 程序占用则报错。
pub fn start_dsh(app: &AppHandle) -> Result<DshStatus, String> {
    let cfg = config::load(app);
    let port = cfg.port;
    match probe(port, Duration::from_secs(2)) {
        Probe::Healthy => return Ok(refresh_status(app)),
        Probe::PortOpen => {
            return Err(format!("端口 {port} 已被其他程序占用（响应不是 DSH）"));
        }
        _ => {}
    }

    let cmdline = dsh_command_line(&cfg);
    let wd = config::work_dir(app);

    let state = app.state::<Mutex<DshState>>();
    let mut st = state.lock().unwrap();
    if st.child.is_some() {
        return Ok(status_snapshot(&st, port));
    }

    let mut cmd = build_shell_command(&cmdline);
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    if wd.exists() {
        cmd.current_dir(&wd);
    }
    let mut child = cmd.spawn().map_err(|e| format!("启动 DSH 失败: {e}"))?;
    let pid = child.id();
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    st.child = Some(child);
    st.kind = STATUS_STARTING.to_string();
    st.pid = Some(pid);
    st.started_at = Some(now_ms());
    st.message = Some("正在启动…".to_string());
    drop(st);

    if let Some(o) = stdout {
        spawn_dsh_reader(app.clone(), o, "stdout");
    }
    if let Some(e) = stderr {
        spawn_dsh_reader(app.clone(), e, "stderr");
    }
    push_log(app, format!("$ {cmdline}"));
    spawn_dsh_watcher(app.clone(), port);
    emit_status(app);
    Ok(status_of(app, port))
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

fn stop_dsh_inner(app: &AppHandle, reason: Option<&str>) {
    let port = config::load(app).port;
    let target = {
        let state = app.state::<Mutex<DshState>>();
        let mut st = state.lock().unwrap();
        if let Some(msg) = reason {
            st.message = Some(msg.to_string());
        }
        match st.child.as_mut() {
            Some(child) => Some(child.id()),
            None => None,
        }
    }
    .or_else(|| listening_pid(port));

    if let Some(pid) = target {
        kill_tree(pid);
    }

    // 等待端口释放，最多 8 秒
    let deadline = Instant::now() + Duration::from_secs(8);
    while Instant::now() < deadline {
        match probe(port, Duration::from_millis(800)) {
            Probe::Healthy | Probe::PortOpen => std::thread::sleep(Duration::from_millis(250)),
            _ => break,
        }
    }

    let state = app.state::<Mutex<DshState>>();
    let mut st = state.lock().unwrap();
    st.child = None;
    st.kind = STATUS_STOPPED.to_string();
    st.pid = None;
    st.started_at = None;
    if let Some(msg) = reason {
        st.message = Some(msg.to_string());
    } else {
        st.message = None;
    }
}

pub fn stop_dsh(app: &AppHandle) -> Result<DshStatus, String> {
    let port = config::load(app).port;
    stop_dsh_inner(app, None);
    emit_status(app);
    Ok(status_of(app, port))
}

pub fn restart_dsh(app: &AppHandle) -> Result<DshStatus, String> {
    stop_dsh_inner(app, Some("正在重启…"));
    emit_status(app);
    start_dsh(app)
}

fn run_captured(command_line: &str, timeout: Duration) -> Option<String> {
    #[cfg(windows)]
    let mut cmd = {
        let mut c = Command::new("cmd");
        c.arg("/D")
            .arg("/C")
            .arg(format!("chcp 65001 >nul & {command_line}"));
        c.creation_flags(0x0800_0000);
        c
    };
    #[cfg(not(windows))]
    let mut cmd = {
        let mut c = Command::new("sh");
        c.arg("-c").arg(command_line);
        c.process_group(0);
        c
    };
    let Ok(mut child) = cmd.stdout(Stdio::piped()).stderr(Stdio::null()).spawn() else {
        return None;
    };
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) => {
                if start.elapsed() > timeout {
                    let _ = child.kill();
                    return None;
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(_) => return None,
        }
    }
    let out = child.wait_with_output().ok()?;
    let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

fn spawn_version_fetch(app: &AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || {
        let cfg = config::load(&app);
        let pkg = match &cfg.dsh_version {
            Some(v) if !v.trim().is_empty() => format!("@deepseek-ai/dsh@{}", v.trim()),
            _ => "@deepseek-ai/dsh".to_string(),
        };
        let line = if global_dsh_exists() {
            "dsh --version".to_string()
        } else {
            format!("npx -y {pkg} --version")
        };
        let Some(version) = run_captured(&line, Duration::from_secs(30)) else {
            return;
        };
        if version.is_empty() {
            return;
        }
        let state = app.state::<Mutex<DshState>>();
        let mut st = state.lock().unwrap();
        st.version = Some(version);
        drop(st);
        emit_status(&app);
    });
}
