use std::sync::Arc;

use client::{Client, UserStore};
use gpui::{Entity, IntoElement, ParentElement};
use ui::prelude::*;

/// 에디트 예측 온보딩 UI — Zed Pro 유도 경로(`ZedAiOnboarding`) 제거 이후 Copilot 설정 안내만 노출한다.
pub struct EditPredictionOnboarding {
    _user_store: Entity<UserStore>,
    _client: Arc<Client>,
    copilot_is_configured: bool,
    continue_with_copilot: Arc<dyn Fn(&mut Window, &mut App)>,
}

impl EditPredictionOnboarding {
    pub fn new(
        user_store: Entity<UserStore>,
        client: Arc<Client>,
        copilot_is_configured: bool,
        continue_with_copilot: Arc<dyn Fn(&mut Window, &mut App)>,
        _cx: &mut Context<Self>,
    ) -> Self {
        Self {
            _user_store: user_store,
            _client: client,
            copilot_is_configured,
            continue_with_copilot,
        }
    }
}

impl Render for EditPredictionOnboarding {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_2()
            .child(Label::new(if self.copilot_is_configured {
                "You can continue using GitHub Copilot as your edit prediction provider."
            } else {
                "You can use GitHub Copilot as your edit prediction provider."
            }))
            .child(
                Button::new(
                    "configure-copilot",
                    if self.copilot_is_configured {
                        "Use Copilot"
                    } else {
                        "Configure Copilot"
                    },
                )
                .full_width()
                .style(ButtonStyle::Outlined)
                .on_click({
                    let callback = self.continue_with_copilot.clone();
                    move |_, window, cx| callback(window, cx)
                }),
            )
    }
}
