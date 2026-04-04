// 프롬프트 등록/편집 모달
// 프롬프트 텍스트, 설명, 카테고리를 입력받아 저장한다.

use crate::prompt_store::{
    PromptEntry, add_prompt, load_prompts, remove_prompt, save_prompts, update_prompt,
};
use gpui::{App, Context, DismissEvent, Entity, EventEmitter, Focusable, Render, Window};
use ui::{
    Button, ButtonCommon, ButtonStyle, Color, Divider, FluentBuilder, Label, LabelCommon,
    LabelSize, TintColor, prelude::*,
};
use ui_input::InputField;
use util::ResultExt;
use workspace::ModalView;

/// 모달 모드 — 생성 또는 편집
enum FormMode {
    /// 새 프롬프트 생성
    Create,
    /// 기존 프롬프트 편집 (ID 보관)
    Edit(String),
}

/// 프롬프트 등록/편집 모달
pub struct PromptFormModal {
    mode: FormMode,
    /// 프롬프트 텍스트 입력
    prompt_input: Entity<InputField>,
    /// 설명글 입력
    description_input: Entity<InputField>,
    /// 카테고리 입력
    category_input: Entity<InputField>,
    focus_handle: gpui::FocusHandle,
}

impl ModalView for PromptFormModal {}
impl EventEmitter<DismissEvent> for PromptFormModal {}

impl Focusable for PromptFormModal {
    fn focus_handle(&self, _cx: &App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl PromptFormModal {
    /// 새 프롬프트 생성 모달
    pub fn new_create(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();

        let prompt_placeholder = i18n::t("prompt_palette.prompt_placeholder", cx).to_string();
        let prompt_label = i18n::t("prompt_palette.prompt_text", cx);
        let desc_placeholder = i18n::t("prompt_palette.description_placeholder", cx).to_string();
        let desc_label = i18n::t("prompt_palette.description", cx);
        let cat_placeholder = i18n::t("prompt_palette.category_placeholder", cx).to_string();
        let cat_label = i18n::t("prompt_palette.category", cx);

        let prompt_input = cx.new(|cx| {
            InputField::new(window, cx, &prompt_placeholder)
                .label(prompt_label)
                .tab_index(1)
                .tab_stop(true)
        });
        let description_input = cx.new(|cx| {
            InputField::new(window, cx, &desc_placeholder)
                .label(desc_label)
                .tab_index(2)
                .tab_stop(true)
        });
        let category_input = cx.new(|cx| {
            InputField::new(window, cx, &cat_placeholder)
                .label(cat_label)
                .tab_index(3)
                .tab_stop(true)
        });

        // 첫 번째 필드에 포커스
        prompt_input.focus_handle(cx).focus(window, cx);

        Self {
            mode: FormMode::Create,
            prompt_input,
            description_input,
            category_input,
            focus_handle,
        }
    }

    /// 기존 프롬프트 편집 모달
    pub fn new_edit(entry: PromptEntry, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut modal = Self::new_create(window, cx);
        modal.mode = FormMode::Edit(entry.id);

        // 기존 값 채우기 — borrow 문제를 피하기 위해 클론하여 전달
        let prompt_text = entry.prompt;
        let description_text = entry.description;
        let category_text = entry.category;

        modal
            .prompt_input
            .update(cx, |input, cx| input.set_text(&prompt_text, window, cx));
        modal
            .description_input
            .update(cx, |input, cx| input.set_text(&description_text, window, cx));
        modal
            .category_input
            .update(cx, |input, cx| input.set_text(&category_text, window, cx));

        modal
    }

    /// 저장 처리
    fn save(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let prompt = self.prompt_input.read(cx).text(cx).trim().to_string();
        let description = self.description_input.read(cx).text(cx).trim().to_string();
        let category = self.category_input.read(cx).text(cx).trim().to_string();

        // 프롬프트 텍스트는 필수
        if prompt.is_empty() {
            return;
        }

        let mut collection = load_prompts();

        match &self.mode {
            FormMode::Create => {
                let entry = PromptEntry::new(prompt, description, category);
                add_prompt(&mut collection, entry);
            }
            FormMode::Edit(id) => {
                update_prompt(&mut collection, id, prompt, description, category);
            }
        }

        save_prompts(&collection).log_err();
        cx.emit(DismissEvent);
    }

    /// 삭제 처리 (편집 모드에서만)
    fn delete(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if let FormMode::Edit(id) = &self.mode {
            let mut collection = load_prompts();
            remove_prompt(&mut collection, id);
            save_prompts(&collection).log_err();
        }
        cx.emit(DismissEvent);
    }
}

impl Render for PromptFormModal {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let is_edit = matches!(self.mode, FormMode::Edit(_));

        let title = if is_edit {
            i18n::t("prompt_palette.edit_prompt", cx)
        } else {
            i18n::t("prompt_palette.new_prompt", cx)
        };

        v_flex()
            .key_context("PromptFormModal")
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
            .child(Divider::horizontal())
            // 입력 필드들
            .child(self.prompt_input.clone())
            .child(self.description_input.clone())
            .child(self.category_input.clone())
            .child(Divider::horizontal())
            // 버튼 영역
            .child(
                h_flex()
                    .w_full()
                    .justify_between()
                    .child(
                        // 삭제 버튼 (편집 모드에서만 표시)
                        h_flex().when(is_edit, |this| {
                            this.child(
                                Button::new(
                                    "delete",
                                    i18n::t("prompt_palette.delete_prompt", cx),
                                )
                                .style(ButtonStyle::Tinted(TintColor::Error))
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.delete(window, cx);
                                })),
                            )
                        }),
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .child(
                                Button::new("cancel", i18n::t("prompt_palette.cancel", cx))
                                    .style(ButtonStyle::Subtle)
                                    .on_click(cx.listener(|_this, _, _window, cx| {
                                        cx.emit(DismissEvent);
                                    })),
                            )
                            .child(
                                Button::new("save", i18n::t("prompt_palette.save", cx))
                                    .style(ButtonStyle::Filled)
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.save(window, cx);
                                    })),
                            ),
                    ),
            )
    }
}
