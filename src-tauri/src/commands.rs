use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager};

use crate::config::{self, AppConfig};
use crate::WindowRegistry;
use crate::process::{self, DshStatus};
use crate::term;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[tauri::command]
pub fn get_config(app: AppHandle) -> AppConfig {
    config::load(&app)
}

#[tauri::command]
pub fn set_config(app: AppHandle, cfg: AppConfig) -> Result<AppConfig, String> {
    let mut cfg = cfg;
    if cfg.port == 0 {
        cfg.port = 3080;
    }
    config::save(&app, &cfg)?;
    let _ = app.emit("config-changed", &cfg);
    Ok(cfg)
}

#[tauri::command]
pub fn get_dsh_status(app: AppHandle) -> DshStatus {
    process::refresh_status(&app)
}

#[tauri::command]
pub fn dsh_start(app: AppHandle) -> Result<DshStatus, String> {
    process::start_dsh(&app)
}

#[tauri::command]
pub fn dsh_stop(app: AppHandle) -> Result<DshStatus, String> {
    process::stop_dsh(&app)
}

#[tauri::command]
pub fn dsh_restart(app: AppHandle) -> Result<DshStatus, String> {
    process::restart_dsh(&app)
}

#[tauri::command]
pub fn get_dsh_log(app: AppHandle) -> Vec<String> {
    process::get_log(&app)
}

#[tauri::command]
pub fn term_run(app: AppHandle, win_id: u32, command: String) -> Result<u64, String> {
    term::run_command(&app, win_id, command)
}

#[tauri::command]
pub fn term_cancel(app: AppHandle, win_id: u32) -> Result<(), String> {
    term::cancel_command(&app, win_id)
}

#[tauri::command]
pub fn open_native_cmd(app: AppHandle, command: Option<String>) -> Result<(), String> {
    term::open_native_cmd(&app, command)
}

#[tauri::command]
pub fn get_work_dir(app: AppHandle) -> String {
    config::work_dir(&app).to_string_lossy().into_owned()
}

#[tauri::command]
pub fn set_work_dir(app: AppHandle, path: String) -> Result<AppConfig, String> {
    let mut cfg = config::load(&app);
    cfg.work_dir = Some(std::path::PathBuf::from(path));
    config::save(&app, &cfg)?;
    let _ = app.emit("config-changed", &cfg);
    Ok(cfg)
}

/// 新建一个 Harnex 窗口（共享同一 DSH 实例）。
#[tauri::command]
pub fn new_window(app: AppHandle, window: tauri::WebviewWindow) -> Result<u32, String> {
    let state = app.state::<Mutex<WindowRegistry>>();
    let id = state.lock().unwrap().alloc();
    // 只有普通 Harnex 窗口作为调用方时才继承尺寸；托盘弹窗等小窗不参与
    let inherit = if window.label().starts_with("harnex-") {
        window
            .inner_size()
            .ok()
            .zip(window.outer_position().ok())
            .map(|(s, p)| (p.x, p.y, s.width, s.height))
    } else {
        None
    };
    let app2 = app.clone();
    crate::spawn_window_creation(app2, id, inherit);
    Ok(id)
}

/// 退出 Harnex（触发退出清理：保存窗口状态、按配置停止 DSH）。

/// 用系统默认浏览器打开外部链接。
#[tauri::command]
pub fn open_url(url: String) -> Result<(), String> {
    #[cfg(windows)]
    {
        let mut cmd = std::process::Command::new("cmd");
        cmd.args(["/C", "start", "", &url]);
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
        cmd.spawn().map_err(|e| format!("打开链接失败: {e}"))?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&url)
            .spawn()
            .map_err(|e| format!("打开链接失败: {e}"))?;
    }
    #[cfg(all(not(windows), not(target_os = "macos")))]
    {
        std::process::Command::new("xdg-open")
            .arg(&url)
            .spawn()
            .map_err(|e| format!("打开链接失败: {e}"))?;
    }
    Ok(())
}
