//! 프롬프트 텍스트의 `{{name}}` placeholder 처리
//!
//! - 매칭 정규식: `(\\)?\{\{(\w+)\}\}` — group 1(앞의 `\`)이 있으면 escape 로 처리
//! - escape: `\{{name}}` 은 placeholder 로 인식되지 않고 출력 시 `{{name}}` 그대로 송신
//! - 사용자 입력 안에 어떤 문자가 들어와도 안전하게 처리 (sentinel 미사용 — 단일 regex 한 번에 escape group 으로 분기)
//!
//! 외부 표준 자료:
//! - handlebars guide: https://handlebarsjs.com/guide/
//! - Mustache spec: https://mustache.github.io/mustache.5.html

use regex::Regex;
use std::collections::HashMap;
use std::sync::OnceLock;

/// `{{name}}` placeholder 매칭 정규식 (lazy 초기화)
/// - group 1: 앞의 `\` (escape 시 Some)
/// - group 2: placeholder 변수명
fn placeholder_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(\\)?\{\{(\w+)\}\}").expect("placeholder regex"))
}

/// 프롬프트 텍스트에서 `{{name}}` placeholder 들을 등장 순서대로 추출
/// - 동일 이름 다중 등장 시 중복 포함 (호출자가 dedup 필요 시 따로 처리)
/// - escape 된 `\{{name}}` 은 추출 대상에서 제외
///
/// 본 함수는 향후 등록 모달의 자동 추출/검증 기능에서 활용 예정. 현재는 단위 테스트에서만 호출.
#[allow(dead_code)]
pub fn extract_placeholders(input: &str) -> Vec<String> {
    placeholder_regex()
        .captures_iter(input)
        .filter_map(|c| {
            // group 1 이 Some 이면 escape 된 placeholder — 추출하지 않음
            if c.get(1).is_some() {
                None
            } else {
                c.get(2).map(|m| m.as_str().to_string())
            }
        })
        .collect()
}

/// 프롬프트 텍스트의 `{{name}}` 을 values 의 값으로 치환
/// - 미정의 변수는 빈 문자열로 치환
/// - `\{{name}}` 은 escape 되어 `{{name}}` 그대로 출력 (앞의 `\` 만 소비)
pub fn apply_arguments(input: &str, values: &HashMap<String, String>) -> String {
    placeholder_regex()
        .replace_all(input, |caps: &regex::Captures| {
            if caps.get(1).is_some() {
                // escape 된 경우 — 매치 전체에서 앞의 `\` 만 제거하고 `{{name}}` 그대로 반환
                let full = caps.get(0).map(|m| m.as_str()).unwrap_or("");
                full.trim_start_matches('\\').to_string()
            } else {
                // 일반 placeholder — values 에서 값을 가져와 치환 (없으면 빈 문자열)
                caps.get(2)
                    .and_then(|m| values.get(m.as_str()).cloned())
                    .unwrap_or_default()
            }
        })
        .into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 헬퍼: 추출 결과 비교
    fn extract(input: &str) -> Vec<String> {
        extract_placeholders(input)
    }

    /// 헬퍼: (key, value) 쌍 슬라이스로 치환
    fn apply(input: &str, values: &[(&str, &str)]) -> String {
        let map: HashMap<String, String> = values
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect();
        apply_arguments(input, &map)
    }

    #[test]
    fn extract_none() {
        assert_eq!(extract("git status"), Vec::<String>::new());
    }

    #[test]
    fn extract_one() {
        assert_eq!(extract("git commit -m {{message}}"), vec!["message"]);
    }

    #[test]
    fn extract_multiple_distinct() {
        assert_eq!(
            extract("docker run -e {{key}}={{value}}"),
            vec!["key", "value"]
        );
    }

    #[test]
    fn extract_multiple_same() {
        assert_eq!(extract("{{n}}+{{n}}"), vec!["n", "n"]);
    }

    #[test]
    fn extract_escape() {
        // \{{name}} 은 escape 되어 placeholder 로 인식되지 않음
        assert_eq!(extract("echo \\{{lit}}"), Vec::<String>::new());
    }

    #[test]
    fn extract_invalid_forms() {
        // 빈 변수명 / 공백 포함 / 비-\w 문자: 매칭 안 됨
        assert_eq!(extract("{{}} {{ x }} {{na-me}}"), Vec::<String>::new());
    }

    #[test]
    fn apply_one() {
        assert_eq!(
            apply("git commit -m {{message}}", &[("message", "hello")]),
            "git commit -m hello"
        );
    }

    #[test]
    fn apply_undefined_to_empty() {
        assert_eq!(apply("git commit -m {{x}}", &[]), "git commit -m ");
    }

    #[test]
    fn apply_escape_preserves() {
        assert_eq!(apply("echo \\{{lit}}", &[]), "echo {{lit}}");
    }

    #[test]
    fn apply_multiple_same_var() {
        assert_eq!(apply("{{n}}+{{n}}={{n}}", &[("n", "1")]), "1+1=1");
    }

    #[test]
    fn apply_mixed_escape_and_placeholder() {
        assert_eq!(
            apply("echo {{name}} \\{{literal}}", &[("name", "world")]),
            "echo world {{literal}}"
        );
    }

    #[test]
    fn apply_no_placeholders_passthrough() {
        assert_eq!(apply("git status", &[]), "git status");
    }

    #[test]
    fn apply_arbitrary_unicode_passthrough() {
        // 어떤 유니코드 문자가 입력에 있어도 placeholder 가 없으면 그대로 통과
        // (이전 sentinel 기반 알고리즘은 SOH 같은 문자가 입력에 있으면 잘못 변환됨 — 현 알고리즘은 안전)
        let input = "a\u{0001}b\u{FFFD}c한글";
        assert_eq!(apply_arguments(input, &HashMap::new()), input);
    }

    #[test]
    fn extract_skips_escaped() {
        // escape 된 placeholder 와 일반 placeholder 가 섞여 있을 때 일반 것만 추출
        assert_eq!(
            extract("echo \\{{lit}} {{name}}"),
            vec!["name"]
        );
    }
}
