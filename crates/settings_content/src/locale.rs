// UI 언어(로케일) 설정 정의

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use settings_macros::MergeFrom;

/// UI에 표시되는 언어를 선택한다.
///
/// `en`은 영어, `ko`는 한국어로 표시한다.
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
    /// English
    #[default]
    En,

    /// 한국어
    Ko,
}
