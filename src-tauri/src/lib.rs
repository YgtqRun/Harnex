mod commands;
mod config;
mod process;
mod term;

use std::sync::Mutex;
use std::time::Duration;
use tauri::{
    image::Image,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager, RunEvent, WindowEvent,
};

use process::DshState;
use term::TermState;

fn toggle_control(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("control") {
        if window.is_visible().unwrap_or(false) {
            let _ = window.hide();
        } else {
            let _ = window.show();
            let _ = window.set_focus();
        }
    }
}

fn build_tray(app: &AppHandle) -> tauri::Result<()> {
    let toggle_item =
        MenuItem::with_id(app, "toggle-control", "显示 / 隐藏控制台", true, None::<&str>)?;
    let start_item = MenuItem::with_id(app, "start-dsh", "启动 DSH", true, None::<&str>)?;
    let stop_item = MenuItem::with_id(app, "stop-dsh", "停止 DSH", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "退出 Harnex", true, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[
            &toggle_item,
            &PredefinedMenuItem::separator(app)?,
            &start_item,
            &stop_item,
            &PredefinedMenuItem::separator(app)?,
            &quit_item,
        ],
    )?;

    let icon = Image::from_bytes(include_bytes!("../icons/icon.png"))?;
    TrayIconBuilder::new()
        .icon(icon)
        .tooltip("Harnex · DeepSeek Harness 桌面壳")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "toggle-control" => toggle_control(app),
            "start-dsh" => {
                let _ = process::start_dsh(app);
            }
            "stop-dsh" => {
                let _ = process::stop_dsh(app);
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                toggle_control(tray.app_handle());
            }
        })
        .build(app)?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(Mutex::new(DshState::default()))
        .manage(Mutex::new(TermState::default()))
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            commands::get_config,
            commands::set_config,
            commands::get_dsh_status,
            commands::dsh_start,
            commands::dsh_stop,
            commands::dsh_restart,
            commands::get_dsh_log,
            commands::term_run,
            commands::term_cancel,
            commands::open_native_cmd,
            commands::get_work_dir,
            commands::set_work_dir,
            commands::show_control,
            commands::hide_control,
            commands::show_main,
        ])
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "main" || window.label() == "control" {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .setup(|app| {
            build_tray(&app.handle())?;

            // 初始状态同步 + 自动拉起 DSH（已在运行则复用）
            process::refresh_status(&app.handle());
            let app2 = app.handle().clone();
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(800));
                let _ = process::start_dsh(&app2);
            });
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            if let RunEvent::ExitRequested { .. } = event {
                let cfg = config::load(app);
                if cfg.stop_on_exit {
                    let _ = process::stop_dsh(app);
                }
            }
        });
}
