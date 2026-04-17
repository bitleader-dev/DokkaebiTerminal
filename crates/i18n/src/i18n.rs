// i18n (국제화) 시스템
// 설정에서 선택한 언어에 따라 UI 문자열을 번역하여 반환한���.

use collections::HashMap;
use gpui::{App, Global, SharedString};
use settings::{Settings, SettingsStore};
use settings_content::{Locale, SettingsContent};

/// 번역 데이터를 보관하는 GPUI 글로벌 리소스
pub struct I18n {
    /// 사용자가 설정한 로케일(`System`/`En`/`Ko`). 드롭다운 상태 유지용.
    locale: Locale,
    /// 실제 번역 lookup에 사용하는 로케일. `locale`이 `System`이면
    /// OS 언어를 감지해 `En`/`Ko` 중 하나로 해석한 값이고, 그 외에는 `locale`과 동일하다.
    effective_locale: Locale,
    /// 로케일별 번역 데이터 (locale -> (key -> value))
    /// 값은 SharedString으로 보관해 매 호출 시 Arc::clone만 발생하도록 한다.
    translations: HashMap<Locale, HashMap<String, SharedString>>,
}

impl Global for I18n {}

/// 설정에서 locale 값을 읽어오는 Settings 구현체
#[derive(Clone, Debug)]
pub struct I18nSettings {
    pub locale: Locale,
}

impl Settings for I18nSettings {
    fn from_settings(content: &SettingsContent) -> Self {
        Self {
            locale: content.locale.unwrap_or_default(),
        }
    }
}

// RegisterSetting 매크로 대신 수동으로 inventory 등록
settings::private::inventory::submit! {
    settings::private::RegisteredSetting {
        settings_value: || {
            Box::new(settings::private::SettingValue::<I18nSettings> {
                global_value: None,
                local_values: Vec::new(),
            })
        },
        from_settings: |content| Box::new(<I18nSettings as Settings>::from_settings(content)),
        id: || std::any::TypeId::of::<I18nSettings>(),
    }
}

impl I18n {
    /// 새 I18n 인스턴스 생성
    fn new() -> Self {
        let locale = Locale::default();
        Self {
            locale,
            effective_locale: resolve_effective_locale(locale),
            translations: HashMap::default(),
        }
    }

    /// 사용자가 설정한 로케일(System 포함) 반환
    pub fn locale(&self) -> Locale {
        self.locale
    }

    /// 실제 번역 lookup에 사용되는 로케일(System을 OS 감지값으로 해석한 결과) 반환
    pub fn effective_locale(&self) -> Locale {
        self.effective_locale
    }

    /// 로케일 변경. `System` 지정 시 OS 언어를 감지해 effective_locale을 함께 갱신한다.
    pub fn set_locale(&mut self, locale: Locale) {
        self.locale = locale;
        self.effective_locale = resolve_effective_locale(locale);
    }

    /// JSON 바이트에서 번역 데이터 로드
    pub fn load_translations(&mut self, locale: Locale, json: &[u8]) {
        if let Ok(map) = serde_json::from_slice::<HashMap<String, String>>(json) {
            let map = map.into_iter().map(|(k, v)| (k, SharedString::from(v))).collect();
            self.translations.insert(locale, map);
        }
    }

    /// 키에 해당하는 번역 문자열 반환. 없으면 키 자체를 반환.
    pub fn translate(&self, key: &str) -> SharedString {
        self.translations
            .get(&self.effective_locale)
            .and_then(|map| map.get(key))
            .cloned()
            .unwrap_or_else(|| SharedString::from(key.to_owned()))
    }
}

/// OS 언어를 감지해 `Locale::System`을 실제 번역 가능한 로케일로 해석한다.
/// `System`이 아닌 값은 그대로 반환된다.
fn resolve_effective_locale(locale: Locale) -> Locale {
    match locale {
        Locale::System => detect_os_locale(),
        other => other,
    }
}

/// OS의 사용자 UI 언어를 조회해 지원 로케일로 매핑한다.
/// BCP-47 태그가 "ko"로 시작하면 `Ko`, 그 외에는 `En`으로 폴백한다.
fn detect_os_locale() -> Locale {
    match sys_locale::get_locale() {
        Some(tag) if tag.to_ascii_lowercase().starts_with("ko") => Locale::Ko,
        _ => Locale::En,
    }
}

/// i18n 시스템 초기화. 앱 시작 시 호출한다.
pub fn init(cx: &mut App) {
    // 설정 등록
    I18nSettings::register(cx);

    // I18n 글로벌 생성 및 번역 파일 로드
    let mut i18n = I18n::new();
    load_translations_from_assets(&mut i18n, cx);

    // 현재 설정의 locale 적용
    let locale = I18nSettings::get_global(cx).locale;
    i18n.set_locale(locale);

    cx.set_global(i18n);

    // settings.json의 locale 값이 바뀌면 I18n 글로벌을 즉시 동기화하고
    // 모든 윈도우를 리페인트해 UI 문자열이 재시작 없이 반영되게 한다.
    cx.observe_global::<SettingsStore>(|cx| {
        let new_locale = I18nSettings::get_global(cx).locale;
        let current_locale = cx.global::<I18n>().locale();
        if new_locale != current_locale {
            cx.global_mut::<I18n>().set_locale(new_locale);
            cx.refresh_windows();
        }
    })
    .detach();
}

/// assets에서 번역 JSON 파일 로드
fn load_translations_from_assets(i18n: &mut I18n, cx: &App) {
    let asset_source = cx.asset_source();

    // 영어 리소스 로드
    if let Ok(Some(data)) = asset_source.load("locales/en.json") {
        i18n.load_translations(Locale::En, &data);
    }

    // 한국어 리소스 로드
    if let Ok(Some(data)) = asset_source.load("locales/ko.json") {
        i18n.load_translations(Locale::Ko, &data);
    }
}

/// 로케일 변경 시 호출. I18n 글로벌의 로케일을 갱신한다.
pub fn set_locale(locale: Locale, cx: &mut App) {
    cx.global_mut::<I18n>().set_locale(locale);
}

/// 현재 로케일 반환
pub fn current_locale(cx: &App) -> Locale {
    cx.global::<I18n>().locale()
}

/// 키에 해당하는 번역 문자열을 반환한다.
pub fn t(key: &str, cx: &App) -> SharedString {
    cx.global::<I18n>().translate(key)
}

/// 단일 `{}` placeholder를 값으로 치환한 번역 문자열을 반환한다.
pub fn t_arg(key: &str, value: impl AsRef<str>, cx: &App) -> String {
    t(key, cx).replace("{}", value.as_ref())
}

/// 명명된 `{name}` placeholder들을 값으로 치환한 번역 문자열을 반환한다.
pub fn t_args(key: &str, args: &[(&str, &str)], cx: &App) -> String {
    let mut result = t(key, cx).to_string();
    for (name, value) in args {
        let pattern = format!("{{{name}}}");
        result = result.replace(&pattern, value);
    }
    result
}
