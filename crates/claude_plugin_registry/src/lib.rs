//! Claude Code 글로벌 설정(`~/.claude/settings.json`) 에 등록되는 Dokkaebi
//! 알림 브리지 플러그인의 식별자와 JSON 편집 로직을 한 곳에 모은 공유 크레이트.
//!
//! cli 언인스톨 경로(`dokkaebi-cli --uninstall-claude-plugin`) 와 앱 내 설정
//! UI(`settings_ui::pages::notification_setup`) 양쪽이 동일한 상수·JSON 구조를
//! 사용하므로 중복을 방지하기 위해 분리. gpui/util 에 의존하지 않아 cli 경로의
//! 의존성 체인을 최소로 유지한다.

use serde_json::Value;
use std::path::PathBuf;

/// 플러그인 식별자. marketplace.json / plugin.json 에 기록된 이름과 반드시 일치.
pub const PLUGIN_NAME: &str = "dokkaebi-notify-bridge";
/// 로컬 디렉터리 marketplace 식별자. `extraKnownMarketplaces` 의 key.
pub const MARKETPLACE_NAME: &str = "dokkaebi-local";
/// `enabledPlugins` 에 등록되는 "플러그인명@marketplace명" 형식의 key.
pub const ENABLED_KEY: &str = "dokkaebi-notify-bridge@dokkaebi-local";

/// Claude Code 글로벌 설정 파일 경로. (`~/.claude/settings.json`)
/// HOME 해석 실패 시 None (이 경우 호출측은 조용히 no-op 처리 권장).
pub fn settings_path() -> Option<PathBuf> {
    dirs::home_dir().map(|home| home.join(".claude").join("settings.json"))
}

/// Claude Code 세션 transcript 루트 경로. (`~/.claude/projects`)
/// transcript tail / 자동 정리 모듈이 공통으로 사용한다.
pub fn projects_root() -> Option<PathBuf> {
    dirs::home_dir().map(|home| home.join(".claude").join("projects"))
}

/// settings.json 을 읽어 JSON 값으로 반환.
/// 파일 부재/읽기 실패/파싱 실패 모두 None 으로 묶어 사용자 파일을 건드리지 않도록 한다.
pub fn read_settings() -> Option<Value> {
    let path = settings_path()?;
    let content = std::fs::read_to_string(&path).ok()?;
    serde_json::from_str(&content).ok()
}

/// settings.json 에 JSON 값을 pretty 형식으로 저장. 저장 성공 여부 반환.
/// 상위 디렉터리(`~/.claude`) 가 없으면 자동 생성한다.
pub fn write_settings(value: &Value) -> bool {
    let Some(path) = settings_path() else {
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

/// 루트 오브젝트의 `key` 항목이 오브젝트이고 비어있으면 제거한다. 제거했으면 true 반환.
fn prune_object_if_empty(root: &mut serde_json::Map<String, Value>, key: &str) -> bool {
    let is_empty = root
        .get(key)
        .and_then(|v| v.as_object())
        .is_some_and(|o| o.is_empty());
    if is_empty {
        root.remove(key);
        true
    } else {
        false
    }
}

/// settings.json 에 등록된 Dokkaebi 알림 브리지 플러그인 항목을 제거한다.
/// 보수적 전략:
/// - `enabledPlugins` 에서 `dokkaebi-notify-bridge@*` 키만 제거
/// - `extraKnownMarketplaces.dokkaebi-local` 만 제거
/// - 위 두 오브젝트가 비게 되면 루트에서도 제거
/// - 다른 사용자 편집 항목은 보존
///
/// 반환값:
/// - `Ok(true)` — 실제로 항목을 제거하고 저장 성공
/// - `Ok(false)` — 파일 부재 / JSON 손상 / 제거 대상 없음. 사용자 파일 미변경
/// - `Err(...)` — 저장 시도 실패
pub fn remove_plugin_registration() -> Result<bool, String> {
    let Some(mut settings) = read_settings() else {
        return Ok(false);
    };
    let Some(root) = settings.as_object_mut() else {
        return Ok(false);
    };

    let mut changed = false;
    let plugin_prefix = format!("{}@", PLUGIN_NAME);

    if let Some(enabled) = root
        .get_mut("enabledPlugins")
        .and_then(|p| p.as_object_mut())
    {
        let before = enabled.len();
        enabled.retain(|k, _| !k.starts_with(&plugin_prefix));
        if enabled.len() != before {
            changed = true;
        }
    }
    changed |= prune_object_if_empty(root, "enabledPlugins");

    if let Some(marketplaces) = root
        .get_mut("extraKnownMarketplaces")
        .and_then(|p| p.as_object_mut())
        && marketplaces.remove(MARKETPLACE_NAME).is_some()
    {
        changed = true;
    }
    changed |= prune_object_if_empty(root, "extraKnownMarketplaces");

    if !changed {
        return Ok(false);
    }

    if !write_settings(&settings) {
        return Err("settings.json 저장 실패".to_string());
    }
    Ok(true)
}

/// `enabledPlugins` 에 `dokkaebi-notify-bridge@*` 키가 존재하는지 확인.
/// 디스크 I/O + JSON 파싱이 발생하므로 렌더 경로에서 반복 호출하는 쪽은
/// 별도 TTL 캐시를 덧입혀 사용한다(호출자 책임).
pub fn is_plugin_installed() -> bool {
    let Some(settings) = read_settings() else {
        return false;
    };
    let plugin_prefix = format!("{}@", PLUGIN_NAME);
    settings
        .get("enabledPlugins")
        .and_then(|p| p.as_object())
        .is_some_and(|plugins| plugins.keys().any(|k| k.starts_with(&plugin_prefix)))
}

/// 본체 빌드 시점의 hooks.json 원본. 컴파일 타임에 임베드되어 본체와 사용자가
/// 설치한 플러그인 디렉터리의 hooks.json 내용 비교에 사용된다.
pub const BUNDLED_HOOKS_JSON: &str =
    include_str!("../../../assets/claude-plugins/dokkaebi-notify-bridge/hooks/hooks.json");

/// 사용자가 설치한 플러그인의 hooks.json 이 본체에 번들된 최신 hooks.json 과
/// 다른지 검사한다. 다르면 본체 업데이트로 hook 정의가 바뀐 것이므로 사용자가
/// 플러그인을 재설치해 새 hooks 를 적용해야 한다.
///
/// 판정 규약:
/// - 미설치 상태(`is_plugin_installed() == false`)면 안내 불필요 → `false`
/// - settings.json 의 `extraKnownMarketplaces.{MARKETPLACE_NAME}.source.path` 누락이거나
///   해당 path 의 hooks.json 을 읽지 못하면 비정상 install 상태 → `true`
/// - 양쪽 hooks.json 을 `serde_json::Value` 로 파싱해 비교. 다르면 `true`,
///   같으면 `false` (whitespace/trailing newline 차이는 무시됨)
///
/// 디스크 I/O + JSON 파싱이 발생하므로 렌더 경로에서 반복 호출하는 쪽은 별도
/// TTL 캐시를 덧입혀 사용한다(호출자 책임).
pub fn needs_reinstall() -> bool {
    if !is_plugin_installed() {
        return false;
    }
    let Some(settings) = read_settings() else {
        return true;
    };
    let path_str = settings
        .get("extraKnownMarketplaces")
        .and_then(|m| m.get(MARKETPLACE_NAME))
        .and_then(|m| m.get("source"))
        .and_then(|s| s.get("path"))
        .and_then(|p| p.as_str());
    let Some(path_str) = path_str else {
        return true;
    };
    let installed_hooks_path = std::path::Path::new(path_str)
        .join(PLUGIN_NAME)
        .join("hooks")
        .join("hooks.json");
    let Ok(installed_content) = std::fs::read_to_string(&installed_hooks_path) else {
        return true;
    };
    let installed_value: Value = match serde_json::from_str(&installed_content) {
        Ok(v) => v,
        Err(_) => return true,
    };
    let bundled_value: Value = match serde_json::from_str(BUNDLED_HOOKS_JSON) {
        Ok(v) => v,
        // 본체 임베드 JSON 이 깨져 있을 가능성은 컴파일 타임에 차단되므로
        // 이 분기는 실질적으로 도달 불가. 도달 시 안내 표시 안 함(보수적).
        Err(_) => return false,
    };
    installed_value != bundled_value
}
