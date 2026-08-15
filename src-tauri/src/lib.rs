mod commands;
mod config;
mod prefs;
mod process;
mod term;

use std::collections::HashSet;
use std::sync::Mutex;
use std::time::Duration;
use tauri::{
    image::Image,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::TrayIconBuilder,
    AppHandle, Listener, Manager, RunEvent, WebviewUrl, WebviewWindowBuilder, WindowEvent,
};

use config::WindowState;
use process::DshState;
use term::TermRegistry;

/// 托盘菜单里需要动态更新的项。
struct TrayItems {
    status: MenuItem<tauri::Wry>,
    start: MenuItem<tauri::Wry>,
    stop: MenuItem<tauri::Wry>,
}

fn status_label(kind: &str) -> &'static str {
    match kind {
        "running" => "运行中",
        "starting" => "启动中…",
        "stopped" => "已停止",
        "portBusy" => "端口被占",
        "error" => "异常",
        _ => "未知",
    }
}

/// 根据 DSH 状态刷新托盘菜单：状态项文字 + 启动/停止置灰。
fn update_tray_status(app: &AppHandle, status: &process::DshStatus) {
    let items = app.state::<Mutex<TrayItems>>();
    let items = items.lock().unwrap();
    let _ = items
        .status
        .set_text(format!("{} · {}", status_label(&status.kind), status.port));
    let running = status.kind == "running" || status.kind == "starting";
    let _ = items.start.set_enabled(!running);
    let _ = items.stop.set_enabled(status.kind != "stopped");
}

/// 已打开窗口的槽位管理：新窗口复用最小可用 id，位置记忆按 id 保存。
#[derive(Default)]
pub struct WindowRegistry {
    active: HashSet<u32>,
}

impl WindowRegistry {
    pub fn alloc(&mut self) -> u32 {
        let mut id = 0u32;
        while self.active.contains(&id) {
            id += 1;
        }
        self.active.insert(id);
        id
    }

    pub fn free(&mut self, id: u32) {
        self.active.remove(&id);
    }
}

fn window_label(id: u32) -> String {
    format!("harnex-{id}")
}

fn window_id_from_label(label: &str) -> Option<u32> {
    label.strip_prefix("harnex-")?.parse().ok()
}

/// 创建（或按记忆恢复）一个 Harnex 窗口；所有窗口共享同一个 DSH 实例。
/// `inherit` 为 Some 时，新窗口继承调用方窗口的尺寸（物理像素），位置做级联偏移。
pub fn create_window(
    app: &AppHandle,
    id: u32,
    inherit: Option<(i32, i32, u32, u32)>,
) -> tauri::Result<tauri::WebviewWindow> {
    let cfg = config::load(app);
    let url = WebviewUrl::App(format!("index.html?id={id}").into());
    let builder = WebviewWindowBuilder::new(app, window_label(id), url)
        .title(format!("Harnex · {}", cfg.port))
        .inner_size(900.0, 700.0)
        .min_inner_size(900.0, 600.0)
        .resizable(true)
        .decorations(false);
    // 物理像素的恢复值，建窗后应用（builder 的 position/inner_size 只接受逻辑值）
    let mut restore: Option<(i32, i32, u32, u32)> = None;
    let mon = app
        .primary_monitor()
        .ok()
        .flatten()
        .map(|m| {
            let s = *m.size();
            (s.width as i32, s.height as i32)
        });
    let (mw, mh) = mon.unwrap_or((1920, 1080));

    if let Some((ix, iy, iw, ih)) = inherit {
        // 新窗口：尺寸跟随调用方窗口，位置级联 +24px 并钳制在屏幕内
        let w = iw.min(mw as u32) as i32;
        let h = ih.min(mh as u32) as i32;
        let x = (ix + 24).clamp(0, (mw - w).max(0));
        let y = (iy + 24).clamp(0, (mh - h).max(0));
        restore = Some((x, y, w as u32, h as u32));
    } else if cfg.remember_window_state {
        if let Some(state) = config::load_window_states(app)
            .into_iter()
            .find(|s| s.id == id)
        {
            // 恢复时按主显示器尺寸钳制，避免历史记录里的最大化尺寸/越界坐标
            let w = state.width.unwrap_or(900).min(mw as u32) as i32;
            let h = state.height.unwrap_or(700).min(mh as u32) as i32;
            let x = state.x.unwrap_or(0).clamp(0, (mw - w).max(0));
            let y = state.y.unwrap_or(0).clamp(0, (mh - h).max(0));
            restore = Some((x, y, w as u32, h as u32));
        }
    }

    let window = builder.build()?;
    if let Some((x, y, w, h)) = restore {
        let _ = window.set_position(tauri::Position::Physical(tauri::PhysicalPosition::new(x, y)));
        let _ = window.set_size(tauri::Size::Physical(tauri::PhysicalSize::new(w, h)));
    }
    window.set_focus()?;
    Ok(window)
}

/// 后台线程稍后创建窗口，避免在 IPC 响应路径内建窗导致 webview 停在 about:blank。
pub fn spawn_window_creation(
    app: AppHandle,
    id: u32,
    inherit: Option<(i32, i32, u32, u32)>,
) {
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(80));
        let _ = create_window(&app, id, inherit);
    });
}

fn save_window_state(app: &AppHandle, id: u32, window: &tauri::Window) {
    if !config::load(app).remember_window_state {
        return;
    }
    // 最大化/最小化时记录的是铺满屏幕的尺寸，跳过保存，避免下次恢复出超大窗口
    if window.is_maximized().unwrap_or(false) || window.is_minimized().unwrap_or(false) {
        return;
    }
    let pos = window.outer_position().ok();
    let size = window.inner_size().ok();
    config::save_window_state(
        app,
        &WindowState {
            id,
            x: pos.map(|p| p.x),
            y: pos.map(|p| p.y),
            width: size.map(|s| s.width),
            height: size.map(|s| s.height),
        },
    );
}

fn build_tray(app: &AppHandle) -> tauri::Result<TrayItems> {
    let status_item =
        MenuItem::with_id(app, "dsh-status", "已停止 · 3080", false, None::<&str>)?;
    let new_item = MenuItem::with_id(app, "new-window", "新建窗口", true, None::<&str>)?;
    let start_item = MenuItem::with_id(app, "start-dsh", "启动 DSH", true, None::<&str>)?;
    let stop_item = MenuItem::with_id(app, "stop-dsh", "停止 DSH", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "退出 Harnex", true, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[
            &status_item,
            &PredefinedMenuItem::separator(app)?,
            &new_item,
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
        .on_menu_event(|app, event| match event.id().as_ref() {
            "new-window" => {
                let state = app.state::<Mutex<WindowRegistry>>();
                let id = state.lock().unwrap().alloc();
                spawn_window_creation(app.clone(), id, None);
            }
            "start-dsh" => {
                let _ = process::start_dsh(app);
            }
            "stop-dsh" => {
                let _ = process::stop_dsh(app);
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;
    Ok(TrayItems {
        status: status_item,
        start: start_item,
        stop: stop_item,
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(Mutex::new(DshState::default()))
        .manage(Mutex::new(TermRegistry::default()))
        .manage(Mutex::new(WindowRegistry::default()))
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
            commands::new_window,
            commands::open_url,
        ])
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { .. } = event {
                let label = window.label().to_string();
                if let Some(id) = window_id_from_label(&label) {
                    let app = window.app_handle();
                    save_window_state(&app, id, window);
                    let state = app.state::<Mutex<WindowRegistry>>();
                    state.lock().unwrap().free(id);
                }
            }
        })
        .setup(|app| {
            let tray_items = build_tray(&app.handle())?;
            app.manage(Mutex::new(tray_items));

            // 状态变化 → 刷新托盘菜单
            let handle = app.handle().clone();
            app.handle().listen("dsh-status", move |event| {
                if let Ok(status) = serde_json::from_str::<process::DshStatus>(event.payload()) {
                    update_tray_status(&handle, &status);
                }
            });

            let state = app.state::<Mutex<WindowRegistry>>();
            let id = state.lock().unwrap().alloc();
            create_window(&app.handle(), id, None)?;

            process::refresh_status(&app.handle());
            let app2 = app.handle().clone();
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(800));
                let _ = process::start_dsh(&app2);
            });
            prefs::spawn_prefs_watcher(app.handle().clone());
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| match event {
            RunEvent::ExitRequested { code, api, .. } => {
                if code.is_none() {
                    // 所有窗口都关闭：保留托盘常驻
                    api.prevent_exit();
                } else {
                    // 主动退出：保存各窗口状态，并按配置决定是否停止 DSH
                    let handle = app.clone();
                    let cfg = config::load(&handle);
                    if cfg.remember_window_state {
                        for (label, window) in handle.webview_windows() {
                            if let Some(id) = window_id_from_label(&label) {
                                if window.is_maximized().unwrap_or(false)
                                    || window.is_minimized().unwrap_or(false)
                                {
                                    continue;
                                }
                                let pos = window.outer_position().ok();
                                let size = window.inner_size().ok();
                                config::save_window_state(
                                    &handle,
                                    &WindowState {
                                        id,
                                        x: pos.map(|p| p.x),
                                        y: pos.map(|p| p.y),
                                        width: size.map(|s| s.width),
                                        height: size.map(|s| s.height),
                                    },
                                );
                            }
                        }
                    }
                    if cfg.stop_on_exit {
                        let _ = process::stop_dsh(&handle);
                    }
                }
            }
            _ => {}
        });
}
