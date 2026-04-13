// 타이틀바 오른쪽에 표시되는 Dokkaebi 업데이트 아이콘.
// GithubUpdater 상태를 관찰해 "업데이트 가능 / 다운로드 중" 상태만 노출한다.
// - UpdateAvailable: 사용자가 클릭하면 설치 파일 다운로드 + 설치 실행 + 앱 종료를 시작한다.
// - Downloading: 다운로드 중임을 시각적으로 표시.
// - 그 외(Idle/Errored): 아이콘 숨김.

use github_update::{GithubUpdateStatus, GithubUpdater};
use gpui::{Empty, Render};
use i18n::t;
use ui::{UpdateButton, prelude::*};

pub struct UpdateVersion {
    status: GithubUpdateStatus,
}

impl UpdateVersion {
    pub fn new(cx: &mut Context<Self>) -> Self {
        // 업데이터가 아직 등록되지 않았거나 이후에 등록될 수도 있으므로 현재 상태로 초기화 후 observe.
        if let Some(updater) = GithubUpdater::get(cx) {
            cx.observe(&updater, |this, updater, cx| {
                this.status = updater.read(cx).status();
                cx.notify();
            })
            .detach();
            Self {
                status: updater.read(cx).status(),
            }
        } else {
            Self {
                status: GithubUpdateStatus::Idle,
            }
        }
    }

    /// 타이틀바 디버그 액션(`SimulateUpdateAvailable`)에서 호출해 업데이트 UI 전환을 테스트한다.
    pub fn update_simulation(&mut self, cx: &mut Context<Self>) {
        if let Some(updater) = GithubUpdater::get(cx) {
            updater.update(cx, |updater, cx| updater.update_simulation(cx));
        }
    }
}

impl Render for UpdateVersion {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        match &self.status {
            GithubUpdateStatus::UpdateAvailable { version, .. } => {
                // 메시지: "업데이트 가능"
                let message: SharedString = t("update.available", cx).into();
                let tooltip: SharedString =
                    format!("{}: v{}", t("update.click_to_install", cx), version).into();
                UpdateButton::new(ui::IconName::Download, message)
                    .tooltip(tooltip)
                    .on_click(cx.listener(|_this, _event, _window, cx| {
                        if let Some(updater) = GithubUpdater::get(cx) {
                            updater.update(cx, |updater, cx| {
                                updater.start_update(cx);
                            });
                        }
                    }))
                    .into_any_element()
            }
            GithubUpdateStatus::Downloading { version } => {
                let message: SharedString = t("update.downloading", cx).into();
                let tooltip: SharedString = format!("v{}", version).into();
                UpdateButton::new(ui::IconName::ArrowCircle, message)
                    .icon_animate(true)
                    .tooltip(tooltip)
                    .into_any_element()
            }
            GithubUpdateStatus::Errored => {
                let message: SharedString = t("update.failed", cx).into();
                let tooltip: SharedString = t("update.failed.tooltip", cx).into();
                UpdateButton::new(ui::IconName::Warning, message)
                    .icon_color(Color::Warning)
                    .tooltip(tooltip)
                    .with_dismiss()
                    .on_dismiss(cx.listener(|_this, _event, _window, cx| {
                        if let Some(updater) = GithubUpdater::get(cx) {
                            updater.update(cx, |updater, cx| updater.dismiss_error(cx));
                        }
                    }))
                    .into_any_element()
            }
            GithubUpdateStatus::Idle => Empty.into_any_element(),
        }
    }
}
