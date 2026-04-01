//! 텔레메트리 모듈 — 서버 전송 비활성화됨.
//! 매크로 호출은 코드 전체에서 사용되므로 no-op으로 유지한다.

/// 텔레메트리 이벤트 매크로 (no-op).
/// 서버 전송이 제거되었으므로 아무 동작도 하지 않는다.
#[macro_export]
macro_rules! event {
    ($($args:tt)*) => { () };
}

/// 하위 호환성을 위해 유지. 아무 동작도 하지 않는다.
#[macro_export]
macro_rules! serialize_property {
    ($key:ident) => {
        $key
    };
    ($key:ident = $value:expr) => {
        $value
    };
}
