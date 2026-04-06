use gpui::{App, ReadGlobal};
use serde_json::Value;
use settings::SettingsStore;
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

/// Claude Code settings.json에 JSON 값을 저장한다.
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

/// Claude Code Stop 훅에 벨 알림(마커 파일 생성)이 설정되어 있는지 확인한다.
fn is_stop_hook_bell_enabled() -> bool {
    let Some(settings) = read_claude_code_settings() else {
        return false;
    };
    let Some(stop_hooks) = settings
        .get("hooks")
        .and_then(|h| h.get("Stop"))
        .and_then(|s| s.as_array())
    else {
        return false;
    };

    stop_hooks
        .iter()
        .any(|entry| is_dokkaebi_hook_entry(entry))
}

/// Dokkaebi 관련 훅 항목인지 판별한다 (현재/이전 모든 형식 매칭).
fn is_dokkaebi_hook_entry(entry: &Value) -> bool {
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

/// Claude Code Stop 훅에 벨 알림을 추가하거나 제거한다.
fn set_stop_hook_bell_enabled(enabled: bool) {
    let mut settings =
        read_claude_code_settings().unwrap_or_else(|| Value::Object(Default::default()));

    let root = settings.as_object_mut().unwrap();

    // 이전 형식 훅 먼저 제거 (마이그레이션)
    if let Some(stop_array) = root
        .get_mut("hooks")
        .and_then(|h| h.get_mut("Stop"))
        .and_then(|s| s.as_array_mut())
    {
        stop_array.retain(|entry| !is_dokkaebi_hook_entry(entry));
    }

    if enabled {
        let hooks_obj = root
            .entry("hooks")
            .or_insert_with(|| Value::Object(Default::default()));
        let hooks_map = hooks_obj.as_object_mut().unwrap();

        let stop_array = hooks_map
            .entry("Stop")
            .or_insert_with(|| Value::Array(Vec::new()));
        let stop_arr = stop_array.as_array_mut().unwrap();

        let bell_hook = serde_json::json!({
            "hooks": [{
                "type": "command",
                "command": "echo bell > \"${TMPDIR:-${TEMP:-/tmp}}/dokkaebi_bell_${DOKKAEBI_TERMINAL_ID}\" 2>/dev/null || true",
                "timeout": 2
            }]
        });
        stop_arr.push(bell_hook);
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

/// Zed 설정의 notification.claude_code_bell 값을 읽어
/// Claude Code settings.json의 Stop 훅과 동기화한다.
pub(crate) fn sync_claude_code_bell_setting(cx: &App) {
    let store = SettingsStore::global(cx);
    let zed_enabled = store
        .raw_user_settings()
        .and_then(|user| {
            user.content
                .notification
                .as_ref()?
                .claude_code_bell
                .as_ref()
                .copied()
        })
        .unwrap_or(false);

    let claude_enabled = is_stop_hook_bell_enabled();

    if zed_enabled != claude_enabled {
        set_stop_hook_bell_enabled(zed_enabled);
    }
}
