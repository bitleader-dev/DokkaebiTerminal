//! Windows 로그인 시 Dokkaebi 자동 실행을 위한 레지스트리 연동.
//!
//! `WorkspaceSettings.auto_start` 값을 `HKEY_CURRENT_USER\Software\Microsoft\Windows\CurrentVersion\Run`
//! 아래 `Dokkaebi` 값에 대응시켜:
//!   - `true` → 현재 실행 파일 경로(따옴표 감쌈)를 레지스트리에 기록 → 다음 로그인부터 자동 실행
//!   - `false` → 해당 값 제거 (없는 경우는 무시)
//!
//! Windows 외 플랫폼에서는 모든 함수가 no-op이므로 호출 측에서 `cfg` 분기를 둘 필요가 없다.

use gpui::App;

/// 앱 부팅 직후 1회 동기화 + 설정 변경 시마다 레지스트리 동기화를 걸어 둔다.
pub(crate) fn init(cx: &mut App) {
    #[cfg(target_os = "windows")]
    windows_impl::init(cx);
    #[cfg(not(target_os = "windows"))]
    let _ = cx;
}

#[cfg(target_os = "windows")]
mod windows_impl {
    use anyhow::{Context as _, Result};
    use gpui::App;
    use settings::{Settings, SettingsStore};
    use workspace::WorkspaceSettings;

    /// 레지스트리 Run 키 경로 및 값 이름.
    const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
    const VALUE_NAME: &str = "Dokkaebi";

    pub(super) fn init(cx: &mut App) {
        // 부팅 시점: 현재 설정 값으로 레지스트리 상태를 맞춘다.
        // (앱 경로가 이동됐을 수 있으므로 ON 상태면 매 기동마다 최신 경로로 재기록.)
        let mut last = WorkspaceSettings::get_global(cx).auto_start;
        apply(last);

        // 설정 변경을 감지해 값이 바뀔 때만 레지스트리를 다시 동기화한다.
        cx.observe_global::<SettingsStore>(move |cx| {
            let current = WorkspaceSettings::get_global(cx).auto_start;
            if current != last {
                last = current;
                apply(current);
            }
        })
        .detach();
    }

    fn apply(enabled: bool) {
        if let Err(err) = try_apply(enabled) {
            log::warn!("auto_start 레지스트리 동기화 실패: {err:?}");
        }
    }

    fn try_apply(enabled: bool) -> Result<()> {
        use windows_registry::CURRENT_USER;

        let key = CURRENT_USER
            .create(RUN_KEY)
            .context("HKCU Run 레지스트리 키 열기/생성 실패")?;

        if enabled {
            let exe = std::env::current_exe().context("현재 실행 파일 경로 조회 실패")?;
            // 공백이 포함된 경로도 올바르게 인식되도록 따옴표로 감싼다.
            let value = format!("\"{}\"", exe.display());
            key.set_string(VALUE_NAME, &value)
                .context("auto_start 레지스트리 값 기록 실패")?;
        } else {
            // 값이 없을 수 있으므로 제거 실패는 무시한다.
            let _ = key.remove_value(VALUE_NAME);
        }
        Ok(())
    }
}
