use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

/// Harnex 应用配置，存于 `app_config_dir/config.json`。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    /// DSH Web GUI 端口（默认 3080）
    pub port: u16,
    /// 命令覆盖：留空时自动探测全局 dsh，找不到则退回 npx
    pub dsh_command: Option<String>,
    /// 命令框与原生 cmd 共享的工作目录
    pub work_dir: Option<PathBuf>,
    /// 退出应用时是否停止 DSH（默认保留运行）
    pub stop_on_exit: bool,
    /// 版本锁定，如 `0.1.0-rc.6`，仅用于 npx 路径
    pub dsh_version: Option<String>,
    /// 是否记住各窗口的大小与位置
    #[serde(default = "default_true")]
    pub remember_window_state: bool,
}

fn default_true() -> bool {
    true
}

impl Default for AppConfig {
    fn default() -> Self {
        let home = std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .unwrap_or_else(|_| String::from("."));
        Self {
            port: 3080,
            dsh_command: None,
            work_dir: Some(PathBuf::from(home)),
            stop_on_exit: false,
            dsh_version: None,
            remember_window_state: true,
        }
    }
}

/// 单窗口的位置尺寸记忆（按窗口槽位 id 保存）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowState {
    pub id: u32,
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

fn config_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|e| format!("无法定位配置目录: {e}"))?;
    std::fs::create_dir_all(&dir).map_err(|e| format!("无法创建配置目录: {e}"))?;
    Ok(dir.join("config.json"))
}

fn window_states_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|e| format!("无法定位配置目录: {e}"))?;
    std::fs::create_dir_all(&dir).map_err(|e| format!("无法创建配置目录: {e}"))?;
    Ok(dir.join("windows.json"))
}

pub fn load(app: &AppHandle) -> AppConfig {
    let default = AppConfig::default();
    match config_path(app) {
        Ok(path) => match std::fs::read_to_string(&path) {
            Ok(text) => serde_json::from_str(&text).unwrap_or(default),
            Err(_) => default,
        },
        Err(_) => default,
    }
}

pub fn save(app: &AppHandle, cfg: &AppConfig) -> Result<(), String> {
    let path = config_path(app)?;
    let text = serde_json::to_string_pretty(cfg).map_err(|e| e.to_string())?;
    std::fs::write(path, text).map_err(|e| format!("写入配置失败: {e}"))
}

pub fn load_window_states(app: &AppHandle) -> Vec<WindowState> {
    match window_states_path(app) {
        Ok(path) => match std::fs::read_to_string(&path) {
            Ok(text) => serde_json::from_str(&text).unwrap_or_default(),
            Err(_) => Vec::new(),
        },
        Err(_) => Vec::new(),
    }
}

/// 按 id 覆盖保存单个窗口的状态。
pub fn save_window_state(app: &AppHandle, state: &WindowState) {
    let Ok(path) = window_states_path(app) else {
        return;
    };
    let mut states = load_window_states(app);
    if let Some(existing) = states.iter_mut().find(|s| s.id == state.id) {
        *existing = state.clone();
    } else {
        states.push(state.clone());
    }
    if let Ok(text) = serde_json::to_string_pretty(&states) {
        let _ = std::fs::write(path, text);
    }
}

/// 解析当前共享工作目录：配置值无效时退回用户主目录。
pub fn work_dir(app: &AppHandle) -> PathBuf {
    let cfg = load(app);
    cfg.work_dir
        .filter(|p| p.exists() && p.is_dir())
        .unwrap_or_else(|| {
            std::env::var("USERPROFILE")
                .or_else(|_| std::env::var("HOME"))
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("."))
        })
}
