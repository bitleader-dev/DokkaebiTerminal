use anyhow::Result;
use serde_json::Value;

use crate::migrations::migrate_settings;

/// HTTP 형 context server 의 deprecated `settings` 키를 마이그레이션 시 자동 제거한다.
/// HTTP context server 는 `url` 필드를 가지며 `settings` 필드를 지원하지 않는다.
/// 다른 종류(stdio, extension)의 context server 는 영향을 받지 않는다.
/// 루트뿐 아니라 플랫폼·릴리즈 채널·profiles 오버라이드 내 `context_servers` 도 모두 처리한다.
pub fn remove_settings_from_http_context_servers(value: &mut Value) -> Result<()> {
    migrate_settings(value, &mut migrate_one)
}

fn migrate_one(obj: &mut serde_json::Map<String, Value>) -> Result<()> {
    if let Some(context_servers) = obj.get_mut("context_servers") {
        if let Some(servers) = context_servers.as_object_mut() {
            for (_, server) in servers.iter_mut() {
                if let Some(server_obj) = server.as_object_mut() {
                    if server_obj.contains_key("url") {
                        server_obj.remove("settings");
                    }
                }
            }
        }
    }
    Ok(())
}
