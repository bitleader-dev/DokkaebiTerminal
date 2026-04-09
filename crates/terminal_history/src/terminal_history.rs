// 터미널 명령어 히스토리 팝업
// 셸 히스토리 파일을 읽어서 Picker 기반 검색 가능한 팝업으로 표시한다.

mod history_source;

use fuzzy::{StringMatch, StringMatchCandidate, match_strings};
use gpui::{App, Context, DismissEvent, Entity, EventEmitter, Focusable, Render, WeakEntity, Window};
use history_source::{HistoryEntry, load_history};
use picker::{Picker, PickerDelegate};
use std::sync::Arc;
use terminal_view::TerminalView;
use ui::{Label, LabelCommon, ListItem, ListItemSpacing, prelude::*};
use util::ResultExt;
use workspace::{ModalView, Workspace};

/// 초기화 — 액션 핸들러 등록
pub fn init(cx: &mut App) {
    cx.observe_new(|workspace: &mut Workspace, _window, _: &mut Context<Workspace>| {
        workspace.register_action(toggle_terminal_history);
    })
    .detach();
}

/// 터미널 히스토리 팝업 토글
fn toggle_terminal_history(
    workspace: &mut Workspace,
    _: &zed_actions::terminal_history::Toggle,
    window: &mut Window,
    cx: &mut Context<Workspace>,
) {
    // 활성 터미널에서 셸 정보 가져오기
    let shell = workspace
        .active_item(cx)
        .and_then(|item| item.act_as::<TerminalView>(cx))
        .map(|tv| tv.read(cx).terminal().read(cx).shell().clone());

    let shell = match shell {
        Some(s) => s,
        None => return, // 활성 터미널이 없으면 무시
    };

    let workspace_handle = cx.entity().downgrade();
    workspace.toggle_modal(window, cx, move |window, cx| {
        let delegate =
            TerminalHistoryDelegate::new(cx.entity().downgrade(), workspace_handle, &shell);
        TerminalHistoryPalette::new(delegate, window, cx)
    });
}

// ─── TerminalHistoryPalette (Picker 래퍼) ──────────────────────

pub struct TerminalHistoryPalette {
    picker: Entity<Picker<TerminalHistoryDelegate>>,
}

impl ModalView for TerminalHistoryPalette {}
impl EventEmitter<DismissEvent> for TerminalHistoryPalette {}

impl Focusable for TerminalHistoryPalette {
    fn focus_handle(&self, cx: &App) -> gpui::FocusHandle {
        self.picker.focus_handle(cx)
    }
}

impl Render for TerminalHistoryPalette {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .key_context("TerminalHistory")
            .w(rems(34.))
            .child(self.picker.clone())
    }
}

impl TerminalHistoryPalette {
    pub fn new(
        delegate: TerminalHistoryDelegate,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let picker = cx.new(|cx| Picker::uniform_list(delegate, window, cx));
        Self { picker }
    }
}

// ─── TerminalHistoryDelegate ───────────────────────────────────

pub struct TerminalHistoryDelegate {
    /// 팔레트 뷰 weak 참조
    palette: WeakEntity<TerminalHistoryPalette>,
    /// 전체 히스토리 항목
    all_entries: Vec<HistoryEntry>,
    /// 퍼지 검색 결과
    matches: Vec<StringMatch>,
    /// 현재 선택 인덱스
    selected_index: usize,
}

impl TerminalHistoryDelegate {
    fn new(
        palette: WeakEntity<TerminalHistoryPalette>,
        _workspace: WeakEntity<Workspace>,
        shell: &util::shell::Shell,
    ) -> Self {
        // 셸 히스토리 파일 읽기 (동기 — 파일 I/O는 빠름)
        let all_entries = load_history(shell);

        // 초기 매치: 전체 항목
        let matches = all_entries
            .iter()
            .enumerate()
            .map(|(ix, entry)| StringMatch {
                candidate_id: ix,
                string: entry.command.clone(),
                positions: Vec::new(),
                score: 0.0,
            })
            .collect();

        Self {
            palette,
            all_entries,
            matches,
            selected_index: 0,
        }
    }

    /// 현재 선택된 명령어 반환
    fn selected_command(&self) -> Option<&str> {
        let m = self.matches.get(self.selected_index)?;
        self.all_entries
            .get(m.candidate_id)
            .map(|e| e.command.as_str())
    }

    /// DismissEvent 전달
    fn dismiss(&self, cx: &mut App) {
        self.palette
            .update(cx, |_, cx| cx.emit(DismissEvent))
            .log_err();
    }
}

impl PickerDelegate for TerminalHistoryDelegate {
    type ListItem = ListItem;

    fn placeholder_text(&self, _window: &mut Window, cx: &mut App) -> Arc<str> {
        i18n::t("terminal_history.search_placeholder", cx).into()
    }

    fn no_matches_text(&self, _window: &mut Window, cx: &mut App) -> Option<SharedString> {
        Some(i18n::t("terminal_history.no_history", cx))
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
        let text = self.selected_command().map(|s| s.to_string());

        // 팝업 닫기
        self.dismiss(cx);

        // defer로 다음 틱에서 텍스트 전송 — 포커스 복원 완료 후 실행
        if let Some(text) = text {
            window.defer(cx, move |window, cx| {
                if let Ok(action) =
                    cx.build_action("terminal::SendText", Some(serde_json::json!(text)))
                {
                    window.dispatch_action(action, cx);
                }
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
        let candidates: Vec<StringMatchCandidate> = self
            .all_entries
            .iter()
            .enumerate()
            .map(|(id, entry)| StringMatchCandidate::new(id, &entry.command))
            .collect();

        cx.spawn_in(window, async move |this, cx| {
            let matches = if query.is_empty() {
                // 검색어 없으면 전체 표시 (최신 순)
                candidates
                    .into_iter()
                    .enumerate()
                    .map(|(index, candidate)| StringMatch {
                        candidate_id: index,
                        string: candidate.string,
                        positions: Vec::new(),
                        score: 0.0,
                    })
                    .collect()
            } else {
                match_strings(
                    &candidates,
                    &query,
                    false,
                    true,
                    500,
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
        _cx: &mut Context<Picker<Self>>,
    ) -> Option<Self::ListItem> {
        let m = self.matches.get(ix)?;
        let entry = self.all_entries.get(m.candidate_id)?;

        Some(
            ListItem::new(format!("history-{ix}"))
                .inset(true)
                .spacing(ListItemSpacing::Sparse)
                .toggle_state(selected)
                .child(
                    Label::new(entry.command.clone())
                        .single_line()
                        .truncate(),
                ),
        )
    }
}
