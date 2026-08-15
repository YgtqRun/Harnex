//! 读取 DSH 宿主设置（默认 `~/.dsh/settings.yaml`）中的主题与语言偏好，
//! 变化时通过 `dsh-theme` / `dsh-locale` 事件推送给外壳，实现「DSH 改配置、外壳跟随」。

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

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalePreference {
    /// "zh" | "en"，空串表示未设置（由前端按浏览器语言回退）
    pub preference: String,
}

struct WatchState {
    last_theme: Option<String>,
    last_locale: Option<String>,
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

/// 从 YAML 文本中提取指定顶层段下的键值，如 `ui-theme:` 段的 `preference:`。
fn parse_value<'a>(content: &'a str, section: &str, key: &str) -> Option<String> {
    let section_prefix = format!("{section}:");
    let key_prefix = format!("{key}:");
    let mut in_section = false;
    for raw in content.lines() {
        let line = raw.trim_start();
        if raw.starts_with(|c: char| !c.is_whitespace()) {
            in_section = line.starts_with(&section_prefix);
            continue;
        }
        if in_section && line.starts_with(&key_prefix) {
            let value = line[key_prefix.len()..]
                .trim()
                .trim_matches(['"', '\'']);
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }
    None
}

pub fn spawn_prefs_watcher(app: AppHandle) {
    let Some(path) = settings_path() else {
        return;
    };
    std::thread::spawn(move || {
        let state = Mutex::new(WatchState {
            last_theme: None,
            last_locale: None,
        });
        loop {
            let content = std::fs::read_to_string(&path).ok();
            let theme = content
                .as_deref()
                .and_then(|c| parse_value(c, "ui-theme", "preference"));
            let locale = content
                .as_deref()
                .and_then(|c| parse_value(c, "locale", "preference"));
            {
                let mut st = state.lock().unwrap();
                if st.last_theme != theme {
                    st.last_theme = theme.clone();
                    let _ = app.emit(
                        "dsh-theme",
                        ThemePreference {
                            preference: theme.unwrap_or_else(|| "system".to_string()),
                        },
                    );
                }
                if st.last_locale != locale {
                    st.last_locale = locale.clone();
                    let _ = app.emit(
                        "dsh-locale",
                        LocalePreference {
                            preference: locale.unwrap_or_default(),
                        },
                    );
                }
            }
            std::thread::sleep(Duration::from_millis(1000));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::parse_value;

    #[test]
    fn parses_theme_system() {
        let yaml = "ui-theme:\n  preference: system\npet:\n  visible: true\n";
        assert_eq!(
            parse_value(yaml, "ui-theme", "preference").as_deref(),
            Some("system")
        );
    }

    #[test]
    fn parses_theme_dark() {
        let yaml = "ui-theme:\n  preference: 'dark'\n";
        assert_eq!(
            parse_value(yaml, "ui-theme", "preference").as_deref(),
            Some("dark")
        );
    }

    #[test]
    fn parses_locale_zh() {
        let yaml = "locale:\n  preference: zh\n";
        assert_eq!(
            parse_value(yaml, "locale", "preference").as_deref(),
            Some("zh")
        );
    }

    #[test]
    fn ignores_other_sections() {
        let yaml = "pet:\n  preference: 160\nlocale:\n  preference: en\n";
        assert_eq!(
            parse_value(yaml, "locale", "preference").as_deref(),
            Some("en")
        );
    }

    #[test]
    fn missing_returns_none() {
        assert_eq!(
            parse_value("pet:\n  visible: true\n", "ui-theme", "preference"),
            None
        );
    }
}
