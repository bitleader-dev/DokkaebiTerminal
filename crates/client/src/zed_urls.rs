//! Contains helper functions for constructing URLs to various Zed-related pages.
//!
//! These URLs will adapt to the configured server URL in order to construct
//! links appropriate for the environment (e.g., by linking to a local copy of
//! zed.dev in development).

use gpui::App;

/// Dokkaebi에는 SaaS 계정 시스템이 없어 빈 URL을 반환한다.
pub fn account_url(_cx: &App) -> String {
    String::new()
}

/// Dokkaebi에는 구독 시스템이 없어 빈 URL을 반환한다.
pub fn upgrade_to_zed_pro_url(_cx: &App) -> String {
    String::new()
}

/// Dokkaebi에는 자체 편집 예측 문서가 없어 빈 URL을 반환한다.
pub fn edit_prediction_docs(_cx: &App) -> String {
    String::new()
}

/// Dokkaebi에는 ACP 레지스트리 블로그가 없어 빈 URL을 반환한다.
pub fn acp_registry_blog(_cx: &App) -> String {
    String::new()
}

pub fn shared_agent_thread_url(session_id: &str) -> String {
    format!("zed://agent/shared/{}", session_id)
}
