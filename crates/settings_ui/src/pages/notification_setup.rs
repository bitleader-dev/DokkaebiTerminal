use claude_plugin_registry::{
    ENABLED_KEY, MARKETPLACE_NAME, PLUGIN_NAME, read_settings, remove_plugin_registration,
    write_settings,
};
use gpui::App;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

// ----------------------------------------------------------------------------
// dokkaebi-notify-bridge 플러그인 설치/제거
// ----------------------------------------------------------------------------
// 상수·JSON I/O·제거 로직은 `claude_plugin_registry` 크레이트에서 공유.
// 본 모듈은 설치 경로(marketplace 디렉터리 탐색)와 렌더 경로 TTL 캐시만 담당.

/// 마켓플레이스 루트 디렉터리 위치를 결정한다.
/// Claude Code의 directory source는 `.claude-plugin/marketplace.json` 카탈로그가
/// 있는 **마켓플레이스 루트**를 기대하므로, 단일 플러그인 디렉터리가 아니라
/// 상위 디렉터리를 반환한다.
///
/// 1순위: 인스톨러로 설치된 환경 — `<exe_dir>/plugins`
/// 2순위: 개발 환경 — `<cwd>/assets/claude-plugins`
///
/// 각 후보는 다음 두 파일이 모두 존재할 때만 유효:
/// - `<root>/.claude-plugin/marketplace.json`
/// - `<root>/dokkaebi-notify-bridge/.claude-plugin/plugin.json`
fn marketplace_root_dir() -> Option<PathBuf> {
    fn is_valid(root: &Path) -> bool {
        root.join(".claude-plugin")
            .join("marketplace.json")
            .is_file()
            && root
                .join(PLUGIN_NAME)
                .join(".claude-plugin")
                .join("plugin.json")
                .is_file()
    }

    if let Ok(exe) = std::env::current_exe()
        && let Some(exe_dir) = exe.parent()
    {
        let installed = exe_dir.join("plugins");
        if is_valid(&installed) {
            return Some(installed);
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        let dev_path = cwd.join("assets").join("claude-plugins");
        if is_valid(&dev_path) {
            return Some(dev_path);
        }
    }
    None
}

/// `is_plugin_installed` 결과 캐시. `PLUGIN_INSTALLED_TTL` 동안 파일 재파싱을
/// 생략한다. 설정 페이지 렌더 1회에서 최대 4회 호출되며 재렌더 빈도가 낮지
/// 않아 매번 `~/.claude/settings.json` 을 읽으면 디스크 I/O + JSON 파싱이
/// 반복된다. install/uninstall 직후에는 `invalidate_plugin_installed_cache()`
/// 로 즉시 무효화해 사용자가 토글 상태가 반영되지 않은 UI 를 보지 않게 한다.
static PLUGIN_INSTALLED_CACHE: Mutex<Option<(Instant, bool)>> = Mutex::new(None);
const PLUGIN_INSTALLED_TTL: Duration = Duration::from_millis(500);

/// `plugin_needs_reinstall` 결과 캐시. 본체 임베드 hooks.json 과 사용자 디스크
/// hooks.json 비교는 install_dir 의 파일 I/O + JSON 파싱이라 동일 TTL 로 가드.
static PLUGIN_REINSTALL_CACHE: Mutex<Option<(Instant, bool)>> = Mutex::new(None);

/// install_plugin / uninstall_plugin 성공 시 호출되어 캐시를 즉시 무효화한다.
/// 호출 누락 시에도 TTL 만료로 수백 ms 내에 자동 반영되지만, 토글 즉시 반영을
/// 위해 변경 경로에서 명시적으로 호출한다.
fn invalidate_plugin_installed_cache() {
    if let Ok(mut cache) = PLUGIN_INSTALLED_CACHE.lock() {
        *cache = None;
    }
    if let Ok(mut cache) = PLUGIN_REINSTALL_CACHE.lock() {
        *cache = None;
    }
}

/// `~/.claude/settings.json`의 `enabledPlugins`에 dokkaebi-notify-bridge 항목이 있는지 확인.
/// TTL 캐시를 통해 렌더 경로에서 반복되는 디스크 I/O + JSON 파싱 비용을 제거한다.
pub fn is_plugin_installed(_cx: &App) -> bool {
    let now = Instant::now();
    if let Ok(mut cache) = PLUGIN_INSTALLED_CACHE.lock() {
        if let Some((at, value)) = *cache
            && now.duration_since(at) < PLUGIN_INSTALLED_TTL
        {
            return value;
        }
        let value = claude_plugin_registry::is_plugin_installed();
        *cache = Some((now, value));
        return value;
    }
    // lock poisoning — 안전한 최후 폴백
    claude_plugin_registry::is_plugin_installed()
}

/// 사용자가 설치한 플러그인의 hooks.json 이 본체에 번들된 최신 hooks.json 과
/// 다르면 true. 본체 업데이트로 hook 정의가 바뀌었음을 사용자에게 안내해 재설치를
/// 유도하는 UI 분기에서 사용한다. 미설치 상태이거나 디스크/JSON 비교 비용은
/// `is_plugin_installed` 와 동일 TTL 로 가드된다.
pub fn plugin_needs_reinstall(_cx: &App) -> bool {
    let now = Instant::now();
    if let Ok(mut cache) = PLUGIN_REINSTALL_CACHE.lock() {
        if let Some((at, value)) = *cache
            && now.duration_since(at) < PLUGIN_INSTALLED_TTL
        {
            return value;
        }
        let value = claude_plugin_registry::needs_reinstall();
        *cache = Some((now, value));
        return value;
    }
    // lock poisoning — 안전한 최후 폴백
    claude_plugin_registry::needs_reinstall()
}

/// 플러그인을 `~/.claude/settings.json`에 등록.
/// - `extraKnownMarketplaces.dokkaebi-local`: 로컬 디렉터리 source
/// - `enabledPlugins.dokkaebi-notify-bridge@dokkaebi-local`: true
pub fn install_plugin() -> Result<(), String> {
    let source_dir = marketplace_root_dir()
        .ok_or_else(|| "플러그인 마켓플레이스 디렉터리를 찾을 수 없습니다".to_string())?;

    let mut settings = read_settings().unwrap_or_else(|| Value::Object(Default::default()));
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

    if !write_settings(&settings) {
        return Err("settings.json 저장 실패".to_string());
    }
    invalidate_plugin_installed_cache();
    Ok(())
}

/// 플러그인을 `~/.claude/settings.json`에서 제거.
/// 실제 JSON 편집은 `claude_plugin_registry::remove_plugin_registration` 에서 수행한다.
pub fn uninstall_plugin() -> Result<(), String> {
    remove_plugin_registration()?;
    invalidate_plugin_installed_cache();
    Ok(())
}
