use gpui::App;
use serde_json::Value;
use std::path::PathBuf;

/// Claude Code 글로벌 설정 파일 경로를 반환한다. (~/.claude/settings.json)
fn claude_code_settings_path() -> Option<PathBuf> {
    dirs::home_dir().map(|home| home.join(".claude").join("settings.json"))
}

/// Claude Code settings.json을 읽어 JSON 값으로 반환한다.
fn read_claude_code_settings() -> Option<Value> {
    let path = claude_code_settings_path()?;
    let content = std::fs::read_to_string(&path).ok()?;
    serde_json::from_str(&content).ok()
}

/// Claude Code settings.json에 JSON 값을 저장한다. 성공 여부 반환.
fn write_claude_code_settings(value: &Value) -> bool {
    let Some(path) = claude_code_settings_path() else {
        return false;
    };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let Ok(content) = serde_json::to_string_pretty(value) else {
        return false;
    };
    std::fs::write(&path, content).is_ok()
}

/// 과거 Dokkaebi가 주입했던 마커 파일 hook 항목인지 판별 (마이그레이션 정리용).
fn is_dokkaebi_marker_hook_entry(entry: &Value) -> bool {
    entry
        .get("hooks")
        .and_then(|h| h.as_array())
        .is_some_and(|hooks| {
            hooks.iter().any(|hook| {
                hook.get("command")
                    .and_then(|c| c.as_str())
                    .is_some_and(|cmd| {
                        cmd.contains("dokkaebi_bell")
                            || (cmd.contains("printf") && cmd.contains("\\a"))
                    })
            })
        })
}

/// 과거 `notification.claude_code_bell` 토글이 `~/.claude/settings.json`에 주입했던
/// 마커 파일 hook을 1회 자동 정리한다. 새 IPC 알림 시스템(dokkaebi-notify-bridge 플러그인)
/// 도입에 따른 마이그레이션 단계.
///
/// 다음 메이저 버전에서 본 함수와 호출 지점을 함께 제거 예정.
pub(crate) fn cleanup_legacy_marker_hook(_cx: &App) {
    let Some(mut settings) = read_claude_code_settings() else {
        return;
    };
    let Some(root) = settings.as_object_mut() else {
        return;
    };
    let Some(stop_array) = root
        .get_mut("hooks")
        .and_then(|h| h.get_mut("Stop"))
        .and_then(|s| s.as_array_mut())
    else {
        return;
    };
    let original_len = stop_array.len();
    stop_array.retain(|entry| !is_dokkaebi_marker_hook_entry(entry));
    if stop_array.len() == original_len {
        return; // 변경 사항 없음 → 디스크 쓰기 skip
    }

    // 빈 배열/객체 정리
    if let Some(hooks_obj) = root.get_mut("hooks").and_then(|h| h.as_object_mut()) {
        if hooks_obj
            .get("Stop")
            .and_then(|s| s.as_array())
            .is_some_and(|arr| arr.is_empty())
        {
            hooks_obj.remove("Stop");
        }
        if hooks_obj.is_empty() {
            root.remove("hooks");
        }
    }

    write_claude_code_settings(&settings);
}

// ----------------------------------------------------------------------------
// dokkaebi-notify-bridge 플러그인 설치/제거
// ----------------------------------------------------------------------------

const PLUGIN_NAME: &str = "dokkaebi-notify-bridge";
const MARKETPLACE_NAME: &str = "dokkaebi-local";
const ENABLED_KEY: &str = "dokkaebi-notify-bridge@dokkaebi-local";

/// 플러그인 source 디렉터리 위치를 결정한다.
/// 1순위: 인스톨러로 설치된 환경 — `<exe_dir>/plugins/dokkaebi-notify-bridge`
/// 2순위: 개발 환경 — `<cwd>/assets/claude-plugins/dokkaebi-notify-bridge`
fn plugin_source_dir() -> Option<PathBuf> {
    if let Ok(exe) = std::env::current_exe()
        && let Some(exe_dir) = exe.parent()
    {
        let installed = exe_dir.join("plugins").join(PLUGIN_NAME);
        if installed.is_dir() {
            return Some(installed);
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        let dev_path = cwd
            .join("assets")
            .join("claude-plugins")
            .join(PLUGIN_NAME);
        if dev_path.is_dir() {
            return Some(dev_path);
        }
    }
    None
}

/// `~/.claude/settings.json`의 `enabledPlugins`에 dokkaebi-notify-bridge 항목이 있는지 확인.
pub fn is_plugin_installed(_cx: &App) -> bool {
    let Some(settings) = read_claude_code_settings() else {
        return false;
    };
    settings
        .get("enabledPlugins")
        .and_then(|p| p.as_object())
        .is_some_and(|plugins| {
            plugins
                .keys()
                .any(|k| k.starts_with(&format!("{}@", PLUGIN_NAME)))
        })
}

/// 플러그인을 `~/.claude/settings.json`에 등록.
/// - `extraKnownMarketplaces.dokkaebi-local`: 로컬 디렉터리 source
/// - `enabledPlugins.dokkaebi-notify-bridge@dokkaebi-local`: true
pub fn install_plugin() -> Result<(), String> {
    let source_dir =
        plugin_source_dir().ok_or_else(|| "플러그인 source 디렉터리를 찾을 수 없습니다".to_string())?;

    let mut settings =
        read_claude_code_settings().unwrap_or_else(|| Value::Object(Default::default()));
    let root = settings
        .as_object_mut()
        .ok_or_else(|| "settings.json 형식 오류".to_string())?;

    // extraKnownMarketplaces.dokkaebi-local 등록
    let marketplaces = root
        .entry("extraKnownMarketplaces")
        .or_insert_with(|| Value::Object(Default::default()))
        .as_object_mut()
        .ok_or_else(|| "extraKnownMarketplaces 형식 오류".to_string())?;
    marketplaces.insert(
        MARKETPLACE_NAME.to_string(),
        serde_json::json!({
            "source": {
                "source": "directory",
                "path": source_dir.to_string_lossy(),
            }
        }),
    );

    // enabledPlugins 등록
    let enabled = root
        .entry("enabledPlugins")
        .or_insert_with(|| Value::Object(Default::default()))
        .as_object_mut()
        .ok_or_else(|| "enabledPlugins 형식 오류".to_string())?;
    enabled.insert(ENABLED_KEY.to_string(), Value::Bool(true));

    if !write_claude_code_settings(&settings) {
        return Err("settings.json 저장 실패".to_string());
    }
    Ok(())
}

/// 플러그인을 `~/.claude/settings.json`에서 제거.
pub fn uninstall_plugin() -> Result<(), String> {
    let Some(mut settings) = read_claude_code_settings() else {
        return Ok(());
    };
    let root = settings
        .as_object_mut()
        .ok_or_else(|| "settings.json 형식 오류".to_string())?;

    // enabledPlugins에서 제거
    if let Some(enabled) = root
        .get_mut("enabledPlugins")
        .and_then(|p| p.as_object_mut())
    {
        enabled.retain(|k, _| !k.starts_with(&format!("{}@", PLUGIN_NAME)));
        if enabled.is_empty() {
            root.remove("enabledPlugins");
        }
    }
    // extraKnownMarketplaces.dokkaebi-local 제거
    if let Some(marketplaces) = root
        .get_mut("extraKnownMarketplaces")
        .and_then(|p| p.as_object_mut())
    {
        marketplaces.remove(MARKETPLACE_NAME);
        if marketplaces.is_empty() {
            root.remove("extraKnownMarketplaces");
        }
    }

    if !write_claude_code_settings(&settings) {
        return Err("settings.json 저장 실패".to_string());
    }
    Ok(())
}
