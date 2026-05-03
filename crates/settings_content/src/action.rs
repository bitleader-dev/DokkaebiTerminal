use std::borrow::Cow;
use std::fmt::{Display, Formatter, Result};

use collections::HashMap;
use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use settings_macros::MergeFrom;

/// 등록된 GPUI 액션의 이름. 예: "editor::Cancel", "workspace::CloseActiveItem".
///
/// `command_aliases` 같은 설정 필드나 keymap 파일 바인딩에서 런타임에 알려진
/// 액션 집합에 대해 JSON-schema 자동완성을 요청할 수 있도록 하는 newtype.
#[derive(Serialize, Deserialize, Default, MergeFrom, Clone, Debug, PartialEq)]
#[serde(transparent)]
pub struct ActionName(String);

/// 스키마의 `deprecationMessage` 필드에 메시지를 채우는 헬퍼
fn add_deprecation(schema: &mut Schema, message: String) {
    schema.insert("deprecationMessage".into(), Value::String(message));
}

/// 스키마의 `description` 필드에 설명을 채우는 헬퍼
fn add_description(schema: &mut Schema, description: &str) {
    schema.insert("description".into(), Value::String(description.to_string()));
}

impl ActionName {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    /// `$defs/ActionName` 에 사용할 JSON 스키마를 생성. 액션별 문서·deprecation
    /// 메타데이터를 attached 한 모든 액션의 anyOf.
    pub fn build_schema<'a>(
        action_names: impl IntoIterator<Item = &'a str>,
        action_documentation: &HashMap<&str, &str>,
        deprecations: &HashMap<&str, &str>,
        deprecation_messages: &HashMap<&str, &str>,
    ) -> Schema {
        let mut alternatives = Vec::new();

        for action_name in action_names {
            let mut entry = json_schema!({
                "type": "string",
                "const": action_name
            });

            if let Some(message) = deprecation_messages.get(action_name) {
                add_deprecation(&mut entry, message.to_string());
            } else if let Some(new_name) = deprecations.get(action_name) {
                add_deprecation(&mut entry, format!("Deprecated, use {new_name}"));
            }

            if let Some(description) = action_documentation.get(action_name) {
                add_description(&mut entry, description);
            }

            alternatives.push(entry);
        }

        json_schema!({ "anyOf": alternatives })
    }
}

impl Display for ActionName {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> Result {
        write!(formatter, "{}", self.0)
    }
}

impl AsRef<str> for ActionName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl JsonSchema for ActionName {
    /// schemars 가 schema 생성 중 이 타입을 만나면 `$defs` 맵에 저장할 이름.
    /// 안정적으로 `"ActionName"` 으로 유지해 `#/$defs/ActionName` 참조와
    /// `util::schemars::replace_subschema` 런타임 swap 이 모두 작동.
    fn schema_name() -> Cow<'static, str> {
        "ActionName".into()
    }

    /// 자리표시자로 `true` 반환.
    ///
    /// 실제 schema(등록된 모든 액션의 anyOf + 문서/deprecation 메타데이터)는
    /// `JsonSchema::json_schema` 가 런타임 컨텍스트를 받지 못하므로 여기서 만들 수 없다.
    /// GPUI 액션 레지스트리에 접근 가능한 호출처에서 `ActionName::build_schema` 로 빌드.
    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        json_schema!(true)
    }
}

/// GPUI 액션과 입력 데이터를 함께 담은 두 요소 JSON 배열.
/// 예: `["pane::ActivateItem", { "index": 0 }]`.
#[derive(Deserialize, Default)]
#[serde(transparent)]
pub struct ActionWithArguments(pub Value);

impl JsonSchema for ActionWithArguments {
    /// schemars 가 이 타입을 `$defs` 맵에 저장할 때 사용할 이름.
    /// `#/$defs/ActionWithArguments` 참조와 `replace_subschema` 런타임 swap 호환을 위해 안정 유지.
    fn schema_name() -> Cow<'static, str> {
        "ActionWithArguments".into()
    }

    /// 자리표시자로 `true` 반환.
    ///
    /// 실제 schema 는 keymap_file::generate_json_schema 에서 런타임 정보가 모일 때 빌드.
    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        json_schema!(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_schema_produces_anyof_of_consts_per_name() {
        let mut action_documentation = HashMap::default();
        let mut deprecations = HashMap::default();
        let mut deprecation_messages = HashMap::default();
        action_documentation.insert("editor::Cancel", "Cancel the current operation.");
        deprecations.insert("workspace::CloseCurrentItem", "workspace::CloseActiveItem");
        deprecation_messages.insert("editor::Explode", "DO NOT USE!");

        let schema = ActionName::build_schema(
            [
                "editor::Cancel",
                "editor::Explode",
                "workspace::CloseCurrentItem",
                "workspace::CloseActiveItem",
            ],
            &action_documentation,
            &deprecations,
            &deprecation_messages,
        );

        let value = schema.to_value();
        let values = value
            .pointer("/anyOf")
            .and_then(|v| v.as_array())
            .expect("anyOf should be present");
        assert_eq!(values.len(), 4);

        let (name, schema_type, description) = (
            values[0].get("const").and_then(Value::as_str),
            values[0].get("type").and_then(Value::as_str),
            values[0].get("description").and_then(Value::as_str),
        );
        assert_eq!(name, Some("editor::Cancel"));
        assert_eq!(schema_type, Some("string"));
        assert_eq!(description, Some("Cancel the current operation."));

        let (name, schema_type, message) = (
            values[1].get("const").and_then(Value::as_str),
            values[1].get("type").and_then(Value::as_str),
            values[1].get("deprecationMessage").and_then(Value::as_str),
        );
        assert_eq!(name, Some("editor::Explode"));
        assert_eq!(schema_type, Some("string"));
        assert_eq!(message, Some("DO NOT USE!"));

        let (name, schema_type, message) = (
            values[2].get("const").and_then(Value::as_str),
            values[2].get("type").and_then(Value::as_str),
            values[2].get("deprecationMessage").and_then(Value::as_str),
        );
        assert_eq!(name, Some("workspace::CloseCurrentItem"));
        assert_eq!(schema_type, Some("string"));
        assert_eq!(message, Some("Deprecated, use workspace::CloseActiveItem"));

        let (name, schema_type) = (
            values[3].get("const").and_then(Value::as_str),
            values[3].get("type").and_then(Value::as_str),
        );
        assert_eq!(name, Some("workspace::CloseActiveItem"));
        assert_eq!(schema_type, Some("string"));
    }
}
