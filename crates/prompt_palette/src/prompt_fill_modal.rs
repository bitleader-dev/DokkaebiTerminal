// 프롬프트 인자 입력 모달
// `{{name}}` placeholder 가 있는 프롬프트 호출 시 인자 값을 받아 치환 후 터미널에 송신한다.
// 인자가 없는 프롬프트는 이 모달을 거치지 않고 즉시 송신되므로 본 모달은 arguments.len() >= 1 만 처리한다.

use crate::placeholder::apply_arguments;
use crate::prompt_store::{PromptArgument, record_usage};
use gpui::{App, Context, DismissEvent, Entity, EventEmitter, Focusable, Render, WeakEntity, Window};
use std::collections::HashMap;
use ui::{
    Button, ButtonCommon, ButtonStyle, Color, Divider, Label, LabelCommon, LabelSize, prelude::*,
};
use ui_input::InputField;
use util::ResultExt;
use workspace::{ModalView, Workspace};

/// 프롬프트 인자 입력 모달
pub struct PromptFillModal {
    /// 원본 프롬프트 entry ID — 사용 기록 갱신 시 식별자로 사용
    entry_id: String,
    /// 원본 프롬프트 텍스트 (`{{name}}` placeholder 포함)
    prompt_template: String,
    /// 인자별 (name, 입력 필드) — 순서는 arguments 정의 순서와 일치
    inputs: Vec<(String, Entity<InputField>)>,
    /// 워크스페이스 핸들 — confirm 시 workspace.update 안에서 dispatch 하기 위해 보유
    /// (모달 dismiss 후 focus chain 이 활성 터미널까지 도달 못 하는 문제 회피)
    workspace: WeakEntity<Workspace>,
    focus_handle: gpui::FocusHandle,
}

impl ModalView for PromptFillModal {}
impl EventEmitter<DismissEvent> for PromptFillModal {}

impl Focusable for PromptFillModal {
    fn focus_handle(&self, _cx: &App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl PromptFillModal {
    /// 새 모달 생성
    /// - entry_id: PromptEntry.id — confirm 시 사용 기록 갱신용
    /// - prompt_template: 치환 대상 원본 프롬프트
    /// - arguments: PromptEntry.arguments (1개 이상이어야 함; 0개는 호출자가 본 모달 띄우지 않음)
    /// - workspace: 송신 시 workspace.update 안에서 dispatch 하기 위한 핸들
    pub fn new(
        entry_id: String,
        prompt_template: String,
        arguments: Vec<PromptArgument>,
        workspace: WeakEntity<Workspace>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let focus_handle = cx.focus_handle();

        let inputs: Vec<(String, Entity<InputField>)> = arguments
            .into_iter()
            .enumerate()
            .map(|(idx, arg)| {
                // 라벨은 description 우선, 비어있으면 변수명 fallback
                let label_text = if arg.description.trim().is_empty() {
                    arg.name.clone()
                } else {
                    arg.description.clone()
                };
                let placeholder = arg.default.clone().unwrap_or_default();
                let input = cx.new(|cx| {
                    InputField::new(window, cx, &placeholder)
                        .label(label_text)
                        .tab_index(idx as isize + 1)
                        .tab_stop(true)
                });
                // 기본값이 있으면 미리 채워서 사용자가 그대로 Enter 가능하게
                if let Some(default) = arg.default {
                    if !default.is_empty() {
                        input.update(cx, |field, cx| field.set_text(&default, window, cx));
                    }
                }
                (arg.name, input)
            })
            .collect();

        // 첫 번째 입력 필드에 포커스
        if let Some((_, first)) = inputs.first() {
            first.focus_handle(cx).focus(window, cx);
        }

        Self {
            entry_id,
            prompt_template,
            inputs,
            workspace,
            focus_handle,
        }
    }

    /// 취소 — modal dismiss 후 prompt_palette 다시 띄움
    fn cancel(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        cx.emit(DismissEvent);
        let workspace = self.workspace.clone();
        window.defer(cx, move |window, cx| {
            crate::open_palette_modal(&workspace, window, cx);
        });
    }

    /// 입력값을 모아 placeholder 치환 후 터미널에 송신
    fn confirm(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let mut values: HashMap<String, String> = HashMap::new();
        for (name, input) in &self.inputs {
            let text = input.read(cx).text(cx);
            values.insert(name.clone(), text);
        }
        let final_text = apply_arguments(&self.prompt_template, &values);
        let workspace = self.workspace.clone();
        let entry_id = self.entry_id.clone();

        // 모달 먼저 닫기 (포커스가 직전 활성 위치로 복원됨)
        cx.emit(DismissEvent);

        // defer 안에서 사용 기록 갱신 + workspace.update 컨텍스트로 dispatch
        // (모달 dismiss 후 focus chain 이 터미널까지 자동 복원되지 않는 경우에도 workspace 가 active pane 으로 action 전달)
        window.defer(cx, move |window, cx| {
            // 사용 기록 갱신 (use_count +1, last_used_at = 현재 시각, 비동기 save)
            record_usage(&entry_id, cx);
            workspace
                .update(cx, |_ws, cx| {
                    if let Ok(action) = cx.build_action(
                        "terminal::SendText",
                        Some(serde_json::json!(final_text)),
                    ) {
                        window.dispatch_action(action, cx);
                    }
                })
                .log_err();
        });
    }
}

impl Render for PromptFillModal {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let title = i18n::t("prompt_palette.fill.title", cx);

        let mut layout = v_flex()
            .key_context("PromptFillModal")
            .elevation_3(cx)
            .w(rems(28.))
            .p_4()
            .gap_3()
            // 제목
            .child(
                Label::new(title)
                    .size(LabelSize::Large)
                    .color(Color::Default),
            )
            .child(Divider::horizontal());

        // 인자별 입력 필드 추가
        for (_, input) in &self.inputs {
            layout = layout.child(input.clone());
        }

        layout
            .child(Divider::horizontal())
            // 취소 + 실행 버튼
            .child(
                h_flex()
                    .w_full()
                    .justify_end()
                    .gap_2()
                    .child(
                        Button::new("cancel", i18n::t("prompt_palette.fill.cancel", cx))
                            .style(ButtonStyle::Subtle)
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.cancel(window, cx);
                            })),
                    )
                    .child(
                        Button::new("confirm", i18n::t("prompt_palette.fill.confirm", cx))
                            .style(ButtonStyle::Filled)
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.confirm(window, cx);
                            })),
                    ),
            )
    }
}
