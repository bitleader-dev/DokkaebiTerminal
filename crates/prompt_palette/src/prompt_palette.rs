// 프롬프트 팔레트
// 터미널에서 단축키로 호출하는 프롬프트 선택 팝업.
// Picker 기반으로 프롬프트 목록을 표시하고 선택 시 터미널에 입력한다.

mod placeholder;
mod prompt_fill_modal;
mod prompt_form_modal;
mod prompt_store;

use fuzzy::{StringMatch, StringMatchCandidate, match_strings};
use gpui::{
    AnyElement, App, Context, DismissEvent, Entity, EventEmitter, Focusable, Render, SharedString,
    WeakEntity, Window,
};
use picker::{Picker, PickerDelegate};
use prompt_store::{PromptCollection, PromptEntry, load_prompts};
use std::sync::Arc;
use ui::{
    Button, ButtonCommon, ButtonStyle, Color, Icon, IconButton, IconName, IconSize, Label,
    LabelCommon, LabelSize, ListItem, ListItemSpacing, prelude::*,
};
use util::ResultExt;
use workspace::{ModalView, Workspace};

/// 프롬프트 팔레트 초기화 — 액션 핸들러 등록
pub fn init(cx: &mut App) {
    cx.observe_new(|workspace: &mut Workspace, _window, _: &mut Context<Workspace>| {
        workspace.register_action(toggle_prompt_palette);
        workspace.register_action(open_new_prompt_modal);
    })
    .detach();
}

/// 프롬프트 팔레트 토글
fn toggle_prompt_palette(
    workspace: &mut Workspace,
    _: &zed_actions::prompt_palette::Toggle,
    window: &mut Window,
    cx: &mut Context<Workspace>,
) {
    let workspace_handle = cx.entity().downgrade();
    workspace.toggle_modal(window, cx, |window, cx| {
        let delegate =
            PromptPaletteDelegate::new(cx.entity().downgrade(), workspace_handle);
        PromptPalette::new(delegate, window, cx)
    });
}

/// 새 프롬프트 등록 모달 열기 — workspace 액션 직접 호출용 (palette 거치지 않음)
fn open_new_prompt_modal(
    workspace: &mut Workspace,
    _: &zed_actions::prompt_palette::NewPrompt,
    window: &mut Window,
    cx: &mut Context<Workspace>,
) {
    workspace.toggle_modal(window, cx, |window, cx| {
        // None: palette 를 거치지 않은 직접 호출이므로 닫을 때 palette 복귀 없음
        prompt_form_modal::PromptFormModal::new_create(None, window, cx)
    });
}

/// 프롬프트 팔레트(picker) 모달을 다시 띄움
/// 등록/편집 모달 또는 파라미터 입력 모달이 닫힌 후 사용자가 목록을 다시 볼 수 있도록 복귀시키는 헬퍼.
pub(crate) fn open_palette_modal(
    workspace: &gpui::WeakEntity<Workspace>,
    window: &mut Window,
    cx: &mut App,
) {
    let workspace_for_delegate = workspace.clone();
    workspace
        .update(cx, |ws, cx| {
            ws.toggle_modal(window, cx, |window, cx| {
                let delegate = PromptPaletteDelegate::new(
                    cx.entity().downgrade(),
                    workspace_for_delegate.clone(),
                );
                PromptPalette::new(delegate, window, cx)
            });
        })
        .log_err();
}

// ─── PromptPalette (Picker 래퍼) ────────────────────────────────

pub struct PromptPalette {
    picker: Entity<Picker<PromptPaletteDelegate>>,
}

impl ModalView for PromptPalette {}
impl EventEmitter<DismissEvent> for PromptPalette {}

impl Focusable for PromptPalette {
    fn focus_handle(&self, cx: &App) -> gpui::FocusHandle {
        self.picker.focus_handle(cx)
    }
}

impl Render for PromptPalette {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .key_context("PromptPalette")
            .w(rems(34.))
            .child(self.picker.clone())
    }
}

impl PromptPalette {
    pub fn new(
        delegate: PromptPaletteDelegate,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        // Picker::list 사용 — render_match 가 description 1줄 + 선택적 태그 줄로 가변 높이라
        // uniform_list(모든 행 동일 높이 가정)는 마지막 항목 잘림 발생. list 는 가변 높이 + 자체 스크롤 지원.
        let picker = cx.new(|cx| Picker::list(delegate, window, cx));
        Self { picker }
    }
}

// ─── PromptPaletteDelegate ──────────────────────────────────────

pub struct PromptPaletteDelegate {
    /// 팔레트 뷰 weak 참조 (DismissEvent emit용)
    palette: WeakEntity<PromptPalette>,
    /// 워크스페이스 weak 참조 (모달 열기용)
    workspace: WeakEntity<Workspace>,
    /// 전체 프롬프트 컬렉션
    collection: PromptCollection,
    /// 퍼지 검색 결과
    matches: Vec<StringMatch>,
    /// 현재 선택 인덱스
    selected_index: usize,
}

impl PromptPaletteDelegate {
    fn new(
        palette: WeakEntity<PromptPalette>,
        workspace: WeakEntity<Workspace>,
    ) -> Self {
        let collection = load_prompts();
        let matches = collection
            .prompts
            .iter()
            .enumerate()
            .map(|(ix, entry)| StringMatch {
                candidate_id: ix,
                string: entry.prompt.clone(),
                positions: Vec::new(),
                score: 0.0,
            })
            .collect();

        Self {
            palette,
            workspace,
            collection,
            matches,
            selected_index: 0,
        }
    }

    /// 현재 선택된 프롬프트 항목 반환
    fn selected_entry(&self) -> Option<&PromptEntry> {
        let m = self.matches.get(self.selected_index)?;
        self.collection.prompts.get(m.candidate_id)
    }

    /// DismissEvent를 팔레트 뷰에 전달
    fn dismiss(&self, cx: &mut App) {
        self.palette
            .update(cx, |_, cx| cx.emit(DismissEvent))
            .log_err();
    }
}

impl PickerDelegate for PromptPaletteDelegate {
    type ListItem = ListItem;

    fn placeholder_text(&self, _window: &mut Window, cx: &mut App) -> Arc<str> {
        i18n::t("prompt_palette.search_placeholder", cx).into()
    }

    fn no_matches_text(&self, _window: &mut Window, cx: &mut App) -> Option<SharedString> {
        Some(i18n::t("prompt_palette.no_prompts", cx))
    }

    fn match_count(&self) -> usize {
        self.matches.len()
    }

    fn selected_index(&self) -> usize {
        self.selected_index
    }

    fn set_selected_index(
        &mut self,
        ix: usize,
        _window: &mut Window,
        _cx: &mut Context<Picker<Self>>,
    ) {
        self.selected_index = ix;
    }

    fn confirm(
        &mut self,
        _secondary: bool,
        window: &mut Window,
        cx: &mut Context<Picker<Self>>,
    ) {
        // 선택된 항목의 prompt + arguments 를 클론으로 캡처
        let Some(entry) = self.selected_entry().cloned() else {
            return;
        };
        let workspace = self.workspace.clone();

        // 팔레트 닫기 (포커스가 터미널로 복원됨)
        self.dismiss(cx);

        if entry.arguments.is_empty() {
            // 인자 없음 — defer 안에서 사용 기록 갱신 후 즉시 송신
            let text = entry.prompt;
            let entry_id = entry.id;
            window.defer(cx, move |window, cx| {
                crate::prompt_store::record_usage(&entry_id, cx);
                if let Ok(action) =
                    cx.build_action("terminal::SendText", Some(serde_json::json!(text)))
                {
                    window.dispatch_action(action, cx);
                }
            });
        } else {
            // 인자 있음 — defer 안에서 PromptFillModal 을 워크스페이스 모달로 띄움
            // 사용 기록 갱신은 fill modal 의 confirm 시점에서 처리(취소 시 미갱신)
            let prompt_template = entry.prompt;
            let arguments = entry.arguments;
            let entry_id = entry.id;
            let workspace_for_fill = workspace.clone();
            window.defer(cx, move |window, cx| {
                workspace_for_fill
                    .clone()
                    .update(cx, |ws, cx| {
                        ws.toggle_modal(window, cx, |window, cx| {
                            crate::prompt_fill_modal::PromptFillModal::new(
                                entry_id.clone(),
                                prompt_template,
                                arguments,
                                workspace_for_fill.clone(),
                                window,
                                cx,
                            )
                        });
                    })
                    .log_err();
            });
        }
    }

    fn dismissed(&mut self, _window: &mut Window, cx: &mut Context<Picker<Self>>) {
        self.dismiss(cx);
    }

    fn update_matches(
        &mut self,
        query: String,
        window: &mut Window,
        cx: &mut Context<Picker<Self>>,
    ) -> gpui::Task<()> {
        let background = cx.background_executor().clone();
        // 프롬프트 + 설명 + tags 를 결합하여 검색 후보 생성
        let candidates: Vec<StringMatchCandidate> = self
            .collection
            .prompts
            .iter()
            .enumerate()
            .map(|(id, entry)| {
                let search_text = format!(
                    "{} {} {}",
                    entry.prompt,
                    entry.description,
                    entry.tags.join(" ")
                );
                StringMatchCandidate::new(id, &search_text)
            })
            .collect();
        // 빈 query 정렬용으로 prompts 의 사용 빈도 정보 클론 (async move 안에 캡처)
        let prompts_for_sort = self.collection.prompts.clone();

        cx.spawn_in(window, async move |this, cx| {
            let matches = if query.is_empty() {
                // 빈 query 시 사용 빈도 기반 자동 정렬: last_used_at desc → use_count desc → 등록 순서
                let mut order: Vec<usize> = (0..candidates.len()).collect();
                order.sort_by(|&a, &b| {
                    crate::prompt_store::compare_by_recency(
                        &prompts_for_sort[a],
                        a,
                        &prompts_for_sort[b],
                        b,
                    )
                });
                order
                    .into_iter()
                    .map(|original_idx| {
                        let c = &candidates[original_idx];
                        StringMatch {
                            candidate_id: original_idx,
                            string: c.string.clone(),
                            positions: Vec::new(),
                            score: 0.0,
                        }
                    })
                    .collect()
            } else {
                match_strings(
                    &candidates,
                    &query,
                    false,
                    true,
                    100,
                    &Default::default(),
                    background,
                )
                .await
            };

            this.update(cx, |this, _cx| {
                this.delegate.matches = matches;
                this.delegate.selected_index = this
                    .delegate
                    .selected_index
                    .min(this.delegate.matches.len().saturating_sub(1));
            })
            .log_err();
        })
    }

    fn render_match(
        &self,
        ix: usize,
        selected: bool,
        _window: &mut Window,
        cx: &mut Context<Picker<Self>>,
    ) -> Option<Self::ListItem> {
        let m = self.matches.get(ix)?;
        let entry = self.collection.prompts.get(m.candidate_id)?;

        // 태그 뱃지들 — 다중 분류 (h_flex 로 가로 나열)
        let tags_badges = if entry.tags.is_empty() {
            None
        } else {
            let mut row = h_flex().gap_1();
            for tag in &entry.tags {
                row = row.child(
                    div()
                        .px_1p5()
                        .py_0p5()
                        .rounded_sm()
                        .bg(cx.theme().colors().element_background)
                        .child(
                            Label::new(tag.clone())
                                .size(LabelSize::XSmall)
                                .color(Color::Muted),
                        ),
                );
            }
            Some(row)
        };

        // 편집 버튼
        let entry_id = entry.id.clone();
        let collection = self.collection.clone();
        let palette = self.palette.clone();
        let workspace = self.workspace.clone();
        let edit_button = IconButton::new(("edit", ix), IconName::Pencil)
            .icon_size(IconSize::Small)
            .icon_color(Color::Muted)
            .on_click(cx.listener(move |_this, _, window, cx| {
                let entry_id = entry_id.clone();
                let collection = collection.clone();
                let workspace = workspace.clone();

                // 팔레트 닫기
                palette
                    .update(cx, |_, cx| cx.emit(DismissEvent))
                    .log_err();

                // 편집 모달 열기 — 닫힐 때 prompt_palette 로 복귀시키기 위해 workspace handle 전달
                if let Some(entry) = collection.prompts.iter().find(|e| e.id == entry_id) {
                    let entry = entry.clone();
                    let return_to_palette = workspace.clone();
                    workspace
                        .update(cx, |workspace, cx| {
                            workspace.toggle_modal(window, cx, |window, cx| {
                                prompt_form_modal::PromptFormModal::new_edit(
                                    entry,
                                    Some(return_to_palette.clone()),
                                    window,
                                    cx,
                                )
                            });
                        })
                        .log_err();
                }
            }));

        Some(
            ListItem::new(format!("prompt-{ix}"))
                .inset(true)
                .spacing(ListItemSpacing::Sparse)
                .toggle_state(selected)
                .child(
                    h_flex()
                        .w_full()
                        .gap_2()
                        .child(
                            v_flex()
                                .flex_grow()
                                .overflow_hidden()
                                // 1줄: 프롬프트 텍스트
                                .child(
                                    Label::new(entry.prompt.clone())
                                        .single_line()
                                        .truncate(),
                                )
                                // 2줄: 설명글 + 카테고리
                                // 2줄: 설명 첫 줄만 발췌 (picker 항목 높이 보호 — 다중 줄 입력은 등록/편집 모달에서만)
                                .child(
                                    Label::new(
                                        entry
                                            .description
                                            .lines()
                                            .next()
                                            .unwrap_or("")
                                            .to_string(),
                                    )
                                    .size(LabelSize::Small)
                                    .color(Color::Muted)
                                    .single_line()
                                    .truncate(),
                                )
                                // 3줄: 태그 뱃지들 — description 과 별도 라인에 가로 배치
                                .children(tags_badges),
                        )
                        .child(edit_button),
                ),
        )
    }

    fn render_footer(
        &self,
        _window: &mut Window,
        cx: &mut Context<Picker<Self>>,
    ) -> Option<AnyElement> {
        let palette = self.palette.clone();
        let workspace = self.workspace.clone();
        Some(
            h_flex()
                .p_2()
                .w_full()
                .justify_end()
                .border_t_1()
                .border_color(cx.theme().colors().border_variant)
                .child(
                    Button::new("new-prompt", i18n::t("prompt_palette.new_prompt", cx))
                        .start_icon(Icon::new(IconName::Plus).size(IconSize::Small))
                        .style(ButtonStyle::Subtle)
                        .on_click(cx.listener(move |_this, _, window, cx| {
                            let workspace = workspace.clone();

                            // 팔레트 닫기
                            palette
                                .update(cx, |_, cx| cx.emit(DismissEvent))
                                .log_err();

                            // 새 프롬프트 모달 열기 — 닫힐 때 prompt_palette 로 복귀
                            let return_to_palette = workspace.clone();
                            workspace
                                .update(cx, |workspace, cx| {
                                    workspace.toggle_modal(window, cx, |window, cx| {
                                        prompt_form_modal::PromptFormModal::new_create(
                                            Some(return_to_palette.clone()),
                                            window,
                                            cx,
                                        )
                                    });
                                })
                                .log_err();
                        })),
                )
                .into_any_element(),
        )
    }
}
