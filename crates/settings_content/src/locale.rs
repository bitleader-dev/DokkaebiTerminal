// UI 언어(로케일) 설정 정의

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use settings_macros::MergeFrom;

/// UI에 표시되는 언어를 선택한다.
///
/// `system`은 OS의 사용자 UI 언어를 감지해 자동 선택,
/// `en`은 영어, `ko`는 한국어로 표시한다.
///
/// `#[strum(serialize = "locale.*")]` 속성은 `VariantNames` 기반 드롭다운이
/// i18n 키(`locale.system|en|ko`)를 통해 표시 이름을 가져오도록 하기 위함이다.
#[derive(
    Debug,
    PartialEq,
    Eq,
    Clone,
    Copy,
    Default,
    Hash,
    Serialize,
    Deserialize,
    JsonSchema,
    MergeFrom,
    strum::VariantArray,
    strum::VariantNames,
)]
#[serde(rename_all = "snake_case")]
pub enum Locale {
    /// OS 언어(사용자 UI 언어)를 자동 감지
    #[default]
    #[strum(serialize = "locale.system")]
    System,

    /// English
    #[strum(serialize = "locale.en")]
    En,

    /// 한국어
    #[strum(serialize = "locale.ko")]
    Ko,
}
