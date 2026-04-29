// 프롬프트 등록/편집 모달
// 프롬프트 텍스트, 설명, 카테고리, arguments(파라미터 목록)를 입력받아 저장한다.

use crate::prompt_store::{
    PromptArgument, PromptEntry, add_prompt, load_prompts, parse_tags_input, remove_prompt,
    save_prompts, update_arguments, update_prompt,
};
use editor::{Editor, EditorEvent};
use gpui::{
    App, Context, DismissEvent, Entity, EventEmitter, Focusable, Render, Subscription, WeakEntity,
    Window,
};
use language::language_settings::SoftWrap;
use markdown::{Markdown, MarkdownElement, MarkdownOptions, MarkdownStyle};
use ui::{
    Button, ButtonCommon, ButtonStyle, Color, Divider, FluentBuilder, Icon, IconButton, IconName,
    Label, LabelCommon, LabelSize, TintColor, prelude::*,
};
use ui_input::InputField;
use util::ResultExt;
use workspace::{ModalView, Workspace};

/// 모달 모드 — 생성 또는 편집
enum FormMode {
    /// 새 프롬프트 생성
    Create,
    /// 기존 프롬프트 편집 (ID 보관)
    Edit(String),
}

/// 단일 argument row 의 입력 필드 묶음
struct ArgumentRow {
    /// 변수명 입력 (`{{name}}` 의 name)
    name: Entity<InputField>,
    /// 설명/라벨 입력
    description: Entity<InputField>,
    /// 기본값 입력 (선택)
    default_value: Entity<InputField>,
}

impl ArgumentRow {
    /// 빈 row 생성
    fn new(window: &mut Window, cx: &mut App) -> Self {
        // i18n placeholder + label 텍스트
        let name_ph = i18n::t("prompt_palette.form.argument_name_placeholder", cx).to_string();
        let desc_ph =
            i18n::t("prompt_palette.form.argument_description_placeholder", cx).to_string();
        let default_ph =
            i18n::t("prompt_palette.form.argument_default_placeholder", cx).to_string();
        let name_label = i18n::t("prompt_palette.form.argument_name_label", cx);
        let desc_label = i18n::t("prompt_palette.form.argument_description_label", cx);
        let default_label = i18n::t("prompt_palette.form.argument_default_label", cx);

        let name = cx.new(|cx| InputField::new(window, cx, &name_ph).label(name_label));
        let description = cx.new(|cx| InputField::new(window, cx, &desc_ph).label(desc_label));
        let default_value =
            cx.new(|cx| InputField::new(window, cx, &default_ph).label(default_label));

        Self {
            name,
            description,
            default_value,
        }
    }

    /// 기존 PromptArgument 값으로 채운 row 생성
    fn from_argument(arg: &PromptArgument, window: &mut Window, cx: &mut App) -> Self {
        let row = Self::new(window, cx);
        row.name.update(cx, |f, cx| f.set_text(&arg.name, window, cx));
        row.description
            .update(cx, |f, cx| f.set_text(&arg.description, window, cx));
        if let Some(default) = &arg.default {
            row.default_value
                .update(cx, |f, cx| f.set_text(default, window, cx));
        }
        row
    }

    /// row 의 입력값으로 PromptArgument 생성. name 비어있으면 None.
    fn to_argument(&self, cx: &App) -> Option<PromptArgument> {
        let name = self.name.read(cx).text(cx).trim().to_string();
        if name.is_empty() {
            return None;
        }
        let description = self.description.read(cx).text(cx).trim().to_string();
        let default_text = self.default_value.read(cx).text(cx).trim().to_string();
        let default = if default_text.is_empty() {
            None
        } else {
            Some(default_text)
        };
        Some(PromptArgument {
            name,
            description,
            default,
        })
    }
}

/// 프롬프트 등록/편집 모달
pub struct PromptFormModal {
    mode: FormMode,
    /// 프롬프트 텍스트 입력
    prompt_input: Entity<InputField>,
    /// 설명글 입력 (다중 줄 가능 — multi-line Editor 사용, 줄바꿈 보존)
    description_input: Entity<Editor>,
    /// 설명글 markdown 미리보기 — description_input 변경 시 실시간 갱신.
    /// mermaid 코드 펜스도 SVG 다이어그램으로 렌더링됨 (v3 Phase 2 인프라 재사용).
    description_preview: Entity<Markdown>,
    /// description_input 의 변경 이벤트 구독 — drop 시 자동 해제
    _description_subscription: Subscription,
    /// 태그 입력 (쉼표 구분 텍스트)
    tags_input: Entity<InputField>,
    /// arguments rows — `+` 버튼으로 추가, 각 row 의 삭제 버튼으로 제거
    argument_rows: Vec<ArgumentRow>,
    /// 모달이 닫힐 때(취소/저장/삭제) prompt_palette 를 다시 띄울지 여부.
    /// `Some` 이면 palette 에서 호출되었으므로 닫을 때 palette 로 복귀, `None` 이면 외부 액션 호출이라 그대로 종료.
    return_to_palette: Option<WeakEntity<Workspace>>,
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
    /// - return_to_palette: `Some` 이면 모달 닫힐 때 prompt_palette 로 복귀
    pub fn new_create(
        return_to_palette: Option<WeakEntity<Workspace>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let focus_handle = cx.focus_handle();

        let prompt_placeholder = i18n::t("prompt_palette.prompt_placeholder", cx).to_string();
        let prompt_label = i18n::t("prompt_palette.prompt_text", cx);
        let desc_placeholder = i18n::t("prompt_palette.description_placeholder", cx).to_string();
        let tags_placeholder = i18n::t("prompt_palette.tags_placeholder", cx).to_string();
        let tags_label = i18n::t("prompt_palette.tags", cx);

        let prompt_input = cx.new(|cx| {
            InputField::new(window, cx, &prompt_placeholder)
                .label(prompt_label)
                .tab_index(1)
                .tab_stop(true)
        });
        // 다중 줄 가능한 multi-line Editor 사용 — 줄바꿈 보존, soft wrap, 거터 비활성
        let description_input = cx.new(|cx| {
            let mut editor = Editor::multi_line(window, cx);
            editor.disable_scrollbars_and_minimap(window, cx);
            editor.set_soft_wrap_mode(SoftWrap::EditorWidth, cx);
            editor.set_show_line_numbers(false, cx);
            editor.set_show_git_diff_gutter(false, cx);
            editor.set_show_code_actions(false, cx);
            editor.set_show_runnables(false, cx);
            editor.set_show_wrap_guides(false, cx);
            editor.set_show_indent_guides(false, cx);
            editor.set_placeholder_text(&desc_placeholder, window, cx);
            editor
        });
        // 설명 markdown 미리보기 — mermaid 토글 활성화 (v3 Phase 2 인프라 재사용)
        let description_preview = cx.new(|cx| {
            Markdown::new_with_options(
                SharedString::default(),
                None,
                None,
                MarkdownOptions {
                    render_mermaid_diagrams: true,
                    ..Default::default()
                },
                cx,
            )
        });
        // description_input 의 buffer 변경을 description_preview 에 실시간 반영
        let _description_subscription =
            cx.subscribe(&description_input, |this, editor, event, cx| {
                if let EditorEvent::BufferEdited = event {
                    let text = editor.read(cx).text(cx);
                    this.description_preview.update(cx, |md, cx| {
                        md.replace(text, cx);
                    });
                }
            });
        let tags_input = cx.new(|cx| {
            InputField::new(window, cx, &tags_placeholder)
                .label(tags_label)
                .tab_index(3)
                .tab_stop(true)
        });

        // 첫 번째 필드에 포커스
        prompt_input.focus_handle(cx).focus(window, cx);

        Self {
            mode: FormMode::Create,
            prompt_input,
            description_input,
            description_preview,
            _description_subscription,
            tags_input,
            argument_rows: Vec::new(),
            return_to_palette,
            focus_handle,
        }
    }

    /// 기존 프롬프트 편집 모달
    pub fn new_edit(
        entry: PromptEntry,
        return_to_palette: Option<WeakEntity<Workspace>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let mut modal = Self::new_create(return_to_palette, window, cx);
        modal.mode = FormMode::Edit(entry.id);

        // 기존 값 채우기
        let prompt_text = entry.prompt;
        let description_text = entry.description;
        // tags 를 다시 쉼표 구분 텍스트로 직렬화 (parse_tags_input 의 역함수)
        let tags_text = entry.tags.join(", ");

        modal
            .prompt_input
            .update(cx, |input, cx| input.set_text(&prompt_text, window, cx));
        // multi-line Editor 의 set_text 시그너처: impl Into<Arc<str>> 받음
        modal
            .description_input
            .update(cx, |editor, cx| editor.set_text(description_text.as_str(), window, cx));
        // description_preview 도 초기 텍스트로 동기화 (subscription 은 BufferEdited 만 받으므로 set_text 직후 명시 호출)
        modal.description_preview.update(cx, |md, cx| {
            md.replace(description_text.clone(), cx);
        });
        modal
            .tags_input
            .update(cx, |input, cx| input.set_text(&tags_text, window, cx));

        // 기존 arguments 를 row 로 채움
        modal.argument_rows = entry
            .arguments
            .iter()
            .map(|arg| ArgumentRow::from_argument(arg, window, cx))
            .collect();

        modal
    }

    /// argument row 추가
    fn add_argument(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.argument_rows.push(ArgumentRow::new(window, cx));
        cx.notify();
    }

    /// 특정 idx 의 argument row 제거
    fn remove_argument(&mut self, idx: usize, cx: &mut Context<Self>) {
        if idx < self.argument_rows.len() {
            self.argument_rows.remove(idx);
            cx.notify();
        }
    }

    /// 모달 dismiss + (필요 시) prompt_palette 다시 띄우기
    fn close_and_return(&self, window: &mut Window, cx: &mut Context<Self>) {
        cx.emit(DismissEvent);
        if let Some(workspace) = self.return_to_palette.clone() {
            window.defer(cx, move |window, cx| {
                crate::open_palette_modal(&workspace, window, cx);
            });
        }
    }

    /// 저장 처리
    fn save(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let prompt = self.prompt_input.read(cx).text(cx).trim().to_string();
        // multi-line Editor 의 text 메서드 — 다중 줄 + 줄바꿈 보존. 양 끝의 공백/줄바꿈만 trim.
        let description = self.description_input.read(cx).text(cx).trim().to_string();
        let tags = parse_tags_input(&self.tags_input.read(cx).text(cx));

        // 프롬프트 텍스트는 필수
        if prompt.is_empty() {
            return;
        }

        // arguments rows 에서 PromptArgument 추출 (name 비어있는 row 는 자동 제외)
        let arguments: Vec<PromptArgument> = self
            .argument_rows
            .iter()
            .filter_map(|row| row.to_argument(cx))
            .collect();

        let mut collection = load_prompts();

        match &self.mode {
            FormMode::Create => {
                let entry = PromptEntry::new(prompt, description, tags).with_arguments(arguments);
                add_prompt(&mut collection, entry);
            }
            FormMode::Edit(id) => {
                update_prompt(&mut collection, id, prompt, description, tags);
                update_arguments(&mut collection, id, arguments);
            }
        }

        save_prompts(&collection).log_err();
        self.close_and_return(window, cx);
    }

    /// 삭제 처리 (편집 모드에서만)
    fn delete(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let FormMode::Edit(id) = &self.mode {
            let mut collection = load_prompts();
            remove_prompt(&mut collection, id);
            save_prompts(&collection).log_err();
        }
        self.close_and_return(window, cx);
    }

    /// 취소 처리
    fn cancel(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.close_and_return(window, cx);
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

        // arguments 섹션 — 라벨 + 사용법 안내 (본문 + 예시 별도 줄) + 각 row + 추가 버튼
        let arguments_label = i18n::t("prompt_palette.form.arguments_label", cx);
        let arguments_help = i18n::t("prompt_palette.form.arguments_help", cx);
        let arguments_help_example = i18n::t("prompt_palette.form.arguments_help_example", cx);
        let add_argument_label = i18n::t("prompt_palette.form.add_argument", cx);
        let remove_argument_tooltip = i18n::t("prompt_palette.form.remove_argument", cx);

        let mut arguments_section = v_flex()
            .gap_2()
            .child(
                Label::new(arguments_label)
                    .size(LabelSize::Default)
                    .color(Color::Default),
            )
            .child(
                // 파라미터 사용법 안내 (본문) — muted 색 + Small 크기
                Label::new(arguments_help)
                    .size(LabelSize::Small)
                    .color(Color::Muted),
            )
            .child(
                // 사용법 예시 — 본문과 별도 줄로 분리해 시각적으로 명확히 구분
                Label::new(arguments_help_example)
                    .size(LabelSize::Small)
                    .color(Color::Muted),
            );

        // 각 row 를 카드 형태로 표시 — 헤더(타이틀 + 삭제 버튼) + 입력 필드 3개 vertical stack
        // 한 줄 가로 배치는 모달 폭 초과로 삭제 버튼 잘림 + 입력 겹침 발생 → 카드 v_flex 로 분리
        for (idx, row) in self.argument_rows.iter().enumerate() {
            let tooltip = remove_argument_tooltip.clone();
            let header = h_flex()
                .w_full()
                .justify_between()
                .items_center()
                .child(
                    Label::new(format!("#{}", idx + 1))
                        .size(LabelSize::Small)
                        .color(Color::Muted),
                )
                .child(
                    IconButton::new(("remove-arg", idx), IconName::Close)
                        .icon_color(Color::Muted)
                        .tooltip({
                            let tt = tooltip.clone();
                            move |_window, cx| ui::Tooltip::simple(tt.clone(), cx)
                        })
                        .on_click(cx.listener(move |this, _, _window, cx| {
                            this.remove_argument(idx, cx);
                        })),
                );

            arguments_section = arguments_section.child(
                v_flex()
                    .gap_2()
                    .p_2()
                    .rounded_md()
                    .border_1()
                    .border_color(cx.theme().colors().border)
                    .child(header)
                    .child(row.name.clone())
                    .child(row.description.clone())
                    .child(row.default_value.clone()),
            );
        }

        arguments_section = arguments_section.child(
            Button::new("add-argument", add_argument_label)
                .end_icon(Icon::new(IconName::Plus))
                .style(ButtonStyle::Subtle)
                .on_click(cx.listener(|this, _, window, cx| {
                    this.add_argument(window, cx);
                })),
        );

        v_flex()
            .key_context("PromptFormModal")
            .elevation_3(cx)
            .w(rems(36.))
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
            // 설명 (multi-line) — 라벨 + border 박스 안에 Editor child + 아래에 markdown 미리보기
            .child(
                v_flex()
                    .gap_1()
                    .child(
                        Label::new(i18n::t("prompt_palette.description", cx))
                            .size(LabelSize::Small)
                            .color(Color::Muted),
                    )
                    .child(
                        div()
                            .min_h(rems(4.))
                            .max_h(rems(10.))
                            .px_2()
                            .py_1()
                            .rounded_md()
                            .border_1()
                            .border_color(cx.theme().colors().border)
                            .child(self.description_input.clone()),
                    )
                    // 미리보기 라벨
                    .child(
                        Label::new(i18n::t("prompt_palette.form.preview_label", cx))
                            .size(LabelSize::Small)
                            .color(Color::Muted),
                    )
                    // markdown 미리보기 박스 — description_input 변경 시 실시간 반영
                    .child(
                        div()
                            .min_h(rems(3.))
                            .max_h(rems(10.))
                            .px_2()
                            .py_1()
                            .rounded_md()
                            .border_1()
                            .border_color(cx.theme().colors().border_variant)
                            .bg(cx.theme().colors().element_background)
                            .child(MarkdownElement::new(
                                self.description_preview.clone(),
                                MarkdownStyle::default(),
                            )),
                    ),
            )
            .child(self.tags_input.clone())
            .child(Divider::horizontal())
            // arguments 섹션
            .child(arguments_section)
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
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.cancel(window, cx);
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
