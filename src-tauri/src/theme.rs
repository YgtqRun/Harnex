//! 读取 DSH 宿主设置（默认 `~/.dsh/settings.yaml`）中的主题偏好，
//! 变化时通过 `dsh-theme` 事件推送给外壳，实现「DSH 改配色、外壳跟随」。

use serde::Serialize;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThemePreference {
    /// "system" | "light" | "dark"，皮肤中心的自定义主题 id 原样透传
    pub preference: String,
}

struct WatchState {
    last: Option<String>,
}

fn dsh_home() -> Option<PathBuf> {
    match std::env::var("DSH_HOME") {
        Ok(h) if !h.trim().is_empty() => Some(PathBuf::from(h)),
        _ => {
            let home = std::env::var("USERPROFILE")
                .or_else(|_| std::env::var("HOME"))
                .ok()?;
            Some(PathBuf::from(home).join(".dsh"))
        }
    }
}

fn settings_path() -> Option<PathBuf> {
    dsh_home().map(|h| h.join("settings.yaml"))
}

/// 从 YAML 文本中提取 `ui-theme:` 段下的 `preference:` 值。
fn parse_preference(content: &str) -> Option<String> {
    let mut in_theme = false;
    for raw in content.lines() {
        let line = raw.trim_start();
        if raw.starts_with(|c: char| !c.is_whitespace()) {
            // 顶层键
            in_theme = line.starts_with("ui-theme:");
            continue;
        }
        if in_theme && line.starts_with("preference:") {
            let value = line["preference:".len()..]
                .trim()
                .trim_matches(['"', '\'']);
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }
    None
}

pub fn spawn_theme_watcher(app: AppHandle) {
    let Some(path) = settings_path() else {
        return;
    };
    std::thread::spawn(move || {
        let state = Mutex::new(WatchState { last: None });
        loop {
            let content = std::fs::read_to_string(&path).ok();
            let current = content.as_deref().and_then(parse_preference);
            {
                let mut st = state.lock().unwrap();
                if st.last != current {
                    st.last = current.clone();
                    let payload = ThemePreference {
                        preference: current.unwrap_or_else(|| "system".to_string()),
                    };
                    let _ = app.emit("dsh-theme", payload);
                }
            }
            std::thread::sleep(Duration::from_millis(1000));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::parse_preference;

    #[test]
    fn parses_system() {
        let yaml = "ui-theme:\n  preference: system\npet:\n  visible: true\n";
        assert_eq!(parse_preference(yaml).as_deref(), Some("system"));
    }

    #[test]
    fn parses_dark() {
        let yaml = "ui-theme:\n  preference: 'dark'\n";
        assert_eq!(parse_preference(yaml).as_deref(), Some("dark"));
    }

    #[test]
    fn ignores_other_sections() {
        let yaml = "pet:\n  preference: 160\nui-theme:\n  preference: light\n";
        assert_eq!(parse_preference(yaml).as_deref(), Some("light"));
    }

    #[test]
    fn missing_returns_none() {
        assert_eq!(parse_preference("pet:\n  visible: true\n"), None);
    }
}
