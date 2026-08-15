use tauri::{AppHandle, Emitter, Manager};

use crate::config::{self, AppConfig};
use crate::process::{self, DshStatus};
use crate::term;

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
pub fn term_run(app: AppHandle, command: String) -> Result<u64, String> {
    term::run_command(&app, command)
}

#[tauri::command]
pub fn term_cancel(app: AppHandle) -> Result<(), String> {
    term::cancel_command(&app)
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

#[tauri::command]
pub fn show_control(app: AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("control") {
        window.show().map_err(|e| e.to_string())?;
        window.set_focus().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn hide_control(app: AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("control") {
        window.hide().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn show_main(app: AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        window.show().map_err(|e| e.to_string())?;
        window.set_focus().map_err(|e| e.to_string())?;
    }
    process::sync_main_window(&app);
    Ok(())
}
