//! Claude Code 서브에이전트 단위로 열리는 워크스페이스 Item.
//!
//! 설정 `claude_code.subagent_panel_position` 값에 따라 활성 pane 을 오른쪽/아래쪽
//! 으로 split 해 그 pane 에 탭을 추가한다. "새 파일"/"새 터미널" 메뉴와 달리
//! 서브에이전트 뷰는 항상 split 으로 열려 터미널과 병렬 표시된다.
//! 이미 열려 있는 탭은 설정 변경으로 이동하지 않는다(plan Q1 확정).

use editor::Editor;
use gpui::{
    App, AppContext, Context, Entity, EventEmitter, FocusHandle, Focusable, IntoElement,
    ParentElement, Render, SharedString, Styled, Subscription, Window, div,
};
use language::language_settings::SoftWrap;
use settings::SettingsStore;
use ui::{Color, Divider, Icon, IconName, Label, LabelCommon, h_flex, v_flex};
use util::{time::duration_alt_display, truncate_and_trailoff};
use workspace::{
    Pane, SplitDirection, Workspace,
    item::{Item, ItemEvent},
};

use crate::state::{
    ClaudeSubagentStore, SubagentId, SubagentPanelPosition, SubagentState, SubagentStatus,
    SubagentStoreEvent, claude_code_settings, snapshot,
};

pub struct ClaudeSubagentView {
    subagent_id: SubagentId,
    focus_handle: FocusHandle,
    /// 로그 + 결과 텍스트를 보여주는 read-only multi-line Editor.
    /// 사용자가 본문을 drag-select 해서 복사할 수 있도록 Label 대신 Editor 사용.
    /// 최초 렌더 시 지연 생성(window 가 render 에서만 가용).
    content_editor: Option<Entity<Editor>>,
    /// 마지막으로 에디터에 세팅한 텍스트. 상태가 바뀌어도 텍스트가 동일하면
    /// set_text 호출을 건너뛰어 불필요한 transact 를 줄인다.
    last_rendered_text: String,
    /// 마지막으로 에디터에 적용한 가로 스크롤바(soft-wrap 반대) 설정.
    /// 설정이 바뀔 때만 set_soft_wrap_mode 를 호출해 불필요한 notify 를 줄인다.
    last_horizontal_scrollbar: Option<bool>,
    _store_subscription: Option<Subscription>,
    /// SettingsStore global 변경 감지용. 가로 스크롤바 토글 즉시 반영.
    _settings_subscription: Subscription,
}

impl ClaudeSubagentView {
    pub fn new(
        subagent_id: SubagentId,
        cx: &mut Context<Self>,
    ) -> Self {
        // Store 이벤트(Updated/Stopped) 를 구독해 리렌더.
        // Started 는 new() 호출 시점 이전에 이미 emit 된 뒤이므로 이 subscription 에
        // 도달하지 않아 별도 처리하지 않는다. entity 를 먼저 꺼내 &App 차용을 해제한 뒤
        // cx.subscribe(&mut) 를 호출한다.
        let entity_opt: Option<Entity<crate::state::StoreInner>> =
            ClaudeSubagentStore::get(cx).map(|s| s.entity());
        let subscription = entity_opt.map(|entity| {
            cx.subscribe(
                &entity,
                move |this: &mut Self, _store, event: &SubagentStoreEvent, cx| {
                    let event_id = match event {
                        SubagentStoreEvent::Started(id)
                        | SubagentStoreEvent::Updated(id)
                        | SubagentStoreEvent::Stopped(id) => id,
                    };
                    if event_id == &this.subagent_id {
                        cx.notify();
                    }
                },
            )
        });

        // 설정이 변경되면(특히 가로 스크롤바 토글) 즉시 리렌더되어 soft_wrap_mode 가
        // 에디터에 반영되도록 SettingsStore global 변경을 관찰.
        let settings_subscription = cx.observe_global::<SettingsStore>(|_this, cx| {
            cx.notify();
        });

        Self {
            subagent_id,
            focus_handle: cx.focus_handle(),
            content_editor: None,
            last_rendered_text: String::new(),
            last_horizontal_scrollbar: None,
            _store_subscription: subscription,
            _settings_subscription: settings_subscription,
        }
    }

    pub fn subagent_id(&self) -> &str {
        &self.subagent_id
    }

    fn state_snapshot(&self, cx: &App) -> Option<SubagentState> {
        snapshot(cx, &self.subagent_id)
    }
}

impl Focusable for ClaudeSubagentView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl EventEmitter<()> for ClaudeSubagentView {}

impl Render for ClaudeSubagentView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // 상태 스냅샷 먼저 확보.
        let state_opt = self.state_snapshot(cx);

        // 본문 텍스트(로그 + 결과) 구성. 상태가 없으면 안내 문구 하나.
        let body_text = match &state_opt {
            Some(state) => build_body_text(state, cx),
            None => i18n::t("claude_subagent.log.empty", cx).to_string(),
        };

        // 에디터 지연 생성. 최초 렌더 1회만.
        if self.content_editor.is_none() {
            let editor = cx.new(|cx| {
                let mut e = Editor::multi_line(window, cx);
                e.set_read_only(true);
                e.set_show_gutter(false, cx);
                e.set_show_line_numbers(false, cx);
                e.set_show_indent_guides(false, cx);
                e.set_show_wrap_guides(false, cx);
                e
            });
            self.content_editor = Some(editor);
        }

        // 본문 텍스트가 변경됐을 때만 에디터에 반영. 공통 케이스(로그 append)는
        // 이전 텍스트 뒤에 꼬리만 붙이는 append-only 연장이므로 전체 set_text 대신
        // buffer.edit 로 Δ 만 삽입한다. 이렇게 하면 O(Δ) 비용 + 사용자 스크롤/선택
        // 유지 효과. 결과 섹션이 바뀌는 소수 케이스(Stopped 전환 등) 에만 set_text 폴백.
        if body_text != self.last_rendered_text {
            if let Some(editor) = &self.content_editor {
                let append_suffix = if !self.last_rendered_text.is_empty()
                    && body_text.starts_with(self.last_rendered_text.as_str())
                {
                    Some(body_text[self.last_rendered_text.len()..].to_string())
                } else {
                    None
                };
                if let Some(suffix) = append_suffix {
                    editor.update(cx, |editor, cx| {
                        if let Some(singleton) = editor.buffer().read(cx).as_singleton() {
                            singleton.update(cx, |buffer, cx| {
                                let end = buffer.len();
                                buffer.edit([(end..end, suffix)], None, cx);
                            });
                        }
                    });
                } else {
                    let text_clone = body_text.clone();
                    editor.update(cx, |editor, cx| {
                        editor.set_text(text_clone, window, cx);
                    });
                }
            }
            self.last_rendered_text = body_text;
        }

        // 가로 스크롤바 설정 반영. on 이면 줄바꿈 끄고 가로 스크롤, off 면 soft-wrap.
        let h_scrollbar = claude_code_settings(cx)
            .and_then(|c| c.subagent_horizontal_scrollbar)
            .unwrap_or(false);
        if self.last_horizontal_scrollbar != Some(h_scrollbar) {
            if let Some(editor) = &self.content_editor {
                let mode = if h_scrollbar {
                    SoftWrap::None
                } else {
                    SoftWrap::EditorWidth
                };
                editor.update(cx, |editor, cx| {
                    editor.set_soft_wrap_mode(mode, cx);
                });
            }
            self.last_horizontal_scrollbar = Some(h_scrollbar);
        }

        let editor_view = self.content_editor.as_ref().expect("editor just created").clone();

        // 헤더(유형/설명/상태/경과) + Divider + 에디터. 헤더는 고정, 본문은 에디터 자체 스크롤.
        let header = match &state_opt {
            Some(state) => render_header(state, cx).into_any_element(),
            None => div().p_4().into_any_element(),
        };

        v_flex()
            .size_full()
            .child(header)
            .child(Divider::horizontal())
            .child(
                div()
                    .flex_grow()
                    .size_full()
                    .child(editor_view),
            )
    }
}

fn status_label_key(status: SubagentStatus) -> &'static str {
    match status {
        SubagentStatus::Running => "claude_subagent.status.running",
        SubagentStatus::Completed => "claude_subagent.status.completed",
        SubagentStatus::Cancelled => "claude_subagent.status.cancelled",
        SubagentStatus::Failed => "claude_subagent.status.failed",
    }
}

/// 유형/설명/상태/경과 헤더 블록. 짧은 메타데이터이므로 Label 로 유지.
/// 드래그 선택이 필요한 본문(로그·결과)은 Editor 로 렌더된다.
fn render_header(state: &SubagentState, cx: &App) -> gpui::Div {
    let status_text = i18n::t(status_label_key(state.status), cx).to_string();
    let elapsed_text = duration_alt_display(state.elapsed());
    let header_type = i18n::t("claude_subagent.header.type", cx).to_string();
    let header_desc = i18n::t("claude_subagent.header.description", cx).to_string();
    let header_status = i18n::t("claude_subagent.header.status", cx).to_string();
    let header_elapsed = i18n::t("claude_subagent.header.elapsed", cx).to_string();

    v_flex()
        .gap_1()
        .p_4()
        .child(
            h_flex()
                .gap_2()
                .child(Label::new(header_type).color(Color::Muted))
                .child(Label::new(state.subagent_type.clone())),
        )
        .child(
            h_flex()
                .gap_2()
                .child(Label::new(header_desc).color(Color::Muted))
                .child(Label::new(state.description.clone())),
        )
        .child(
            h_flex()
                .gap_2()
                .child(Label::new(header_status).color(Color::Muted))
                .child(Label::new(status_text))
                .child(Label::new(header_elapsed).color(Color::Muted))
                .child(Label::new(elapsed_text)),
        )
}

/// 로그 + 결과를 단일 문자열로 결합해 read-only Editor 에 표시할 본문을 만든다.
/// 라벨은 i18n 적용. 로그 항목이 없으면 안내 문구로 대체.
fn build_body_text(state: &SubagentState, cx: &App) -> String {
    let mut buf = String::new();

    // 로그 섹션
    let log_section = i18n::t("claude_subagent.section.log", cx).to_string();
    buf.push_str(&format!("=== {} ===\n", log_section));
    if state.log.is_empty() {
        buf.push_str(&i18n::t("claude_subagent.log.empty", cx).to_string());
        buf.push('\n');
    } else {
        for entry in &state.log {
            buf.push_str(&entry.label);
            if !entry.detail.is_empty() {
                buf.push_str("  ");
                buf.push_str(&entry.detail);
            }
            buf.push('\n');
        }
    }

    // 결과 섹션
    buf.push('\n');
    let result_section = i18n::t("claude_subagent.section.result", cx).to_string();
    buf.push_str(&format!("=== {} ===\n", result_section));
    let result_text = match state.status {
        SubagentStatus::Running => i18n::t("claude_subagent.result.pending", cx).to_string(),
        SubagentStatus::Completed | SubagentStatus::Cancelled | SubagentStatus::Failed => state
            .result
            .clone()
            .unwrap_or_else(|| i18n::t("claude_subagent.result.completed", cx).to_string()),
    };
    buf.push_str(&result_text);

    buf
}

impl Item for ClaudeSubagentView {
    type Event = ();

    fn tab_icon(&self, _window: &Window, _cx: &App) -> Option<Icon> {
        Some(Icon::new(IconName::Sparkle))
    }

    fn tab_content_text(&self, _detail: usize, cx: &App) -> SharedString {
        let Some(state) = self.state_snapshot(cx) else {
            return SharedString::from("Subagent");
        };
        let desc = if state.description.is_empty() {
            state.subagent_type.clone()
        } else {
            state.description.clone()
        };
        // 긴 description 이 탭 바를 잠식하지 않도록 20자에서 말줄임표로 자른다.
        SharedString::from(format!(
            "{} · {}",
            state.subagent_type,
            truncate_and_trailoff(&desc, 20)
        ))
    }

    fn telemetry_event_text(&self) -> Option<&'static str> {
        Some("claude subagent view: open")
    }

    fn to_item_events(_event: &Self::Event, _f: &mut dyn FnMut(ItemEvent)) {}
}

/// 서브에이전트 뷰 탭을 오픈한다.
///
/// `target_group_idx` 는 발신 터미널이 속한 워크스페이스 그룹 인덱스.
/// 활성 그룹과 다르면(사용자가 다른 그룹을 보고 있어도) 발신 그룹 안에서
/// 탭을 부착해 **사용자 시야를 바꾸지 않고** 올바른 그룹에 누적되도록 한다.
///
/// 동작 규약:
/// 1. 기존 동일 id 의 탭이 **타겟 그룹의 어느 pane 에든** 있으면 그 탭을 activate 하고 종료.
/// 2. 타겟 그룹 안에서 **"서브에이전트 전용 pane"**(모든 아이템이 ClaudeSubagentView)
///    을 찾아 그 pane 에 새 탭을 추가. 병렬 Task 호출이 많아져도 탭이 **한 pane**
///    에 모이도록 한다.
/// 3. 없으면 타겟 그룹의 활성 pane 을 `position` 방향으로 split 해 새 pane 을 만들고
///    거기에 탭 추가.
pub fn open_subagent_view(
    subagent_id: SubagentId,
    position: SubagentPanelPosition,
    target_group_idx: usize,
    workspace: &mut Workspace,
    window: &mut Window,
    cx: &mut Context<Workspace>,
) {
    // 1~2. 타겟 그룹의 panes/items 를 단일 패스로 훑으며 (a) 동일 id 탭
    // (b) 서브에이전트 전용 pane 을 동시에 탐지. 두 번 순회하면 pane 수 × item 수
    // 만큼 중복 비용이 들어간다.
    let (existing, subagent_only_pane) = scan_panes(&subagent_id, target_group_idx, workspace, cx);
    if let Some((existing_pane, idx)) = existing {
        existing_pane.update(cx, |pane, cx| {
            pane.activate_item(idx, true, true, window, cx);
        });
        return;
    }

    // 2. 서브에이전트 전용 pane 이 이미 있으면 그곳에 추가. 없으면 타겟 그룹의
    // 활성 pane 을 split. split_pane_in_group 이 활성/비활성 그룹을 모두 다룬다.
    let direction = match position {
        SubagentPanelPosition::Right => SplitDirection::Right,
        SubagentPanelPosition::Bottom => SplitDirection::Down,
    };
    let target_pane = match subagent_only_pane {
        Some(pane) => pane,
        None => match workspace.split_pane_in_group(target_group_idx, direction, window, cx) {
            Some(pane) => pane,
            None => {
                // 그룹 인덱스가 범위를 벗어난 비정상 상황 — 활성 pane 폴백.
                let active_pane = workspace.active_pane().clone();
                workspace.split_pane(active_pane, direction, window, cx)
            }
        },
    };

    let view = cx.new(|cx| ClaudeSubagentView::new(subagent_id, cx));
    target_pane.update(cx, |pane, cx| {
        pane.add_item(Box::new(view), true, true, None, window, cx);
    });
}

/// 지정한 워크스페이스 그룹의 panes/items 를 한 번만 훑어 다음을 동시에 반환한다.
/// - 동일 `subagent_id` 탭이 이미 존재하면 그 (pane, index)
/// - 모든 아이템이 `ClaudeSubagentView` 인 첫 pane (서브에이전트 전용 pane 재사용용)
///
/// 병렬 Task 호출로 서브에이전트가 여러 개 동시 실행돼도 모든 탭이 같은 pane 에
/// 모이도록 하는 용도라 active_pane 여부와 무관하게 그룹 내 전 pane 을 검사한다.
fn scan_panes(
    subagent_id: &str,
    target_group_idx: usize,
    workspace: &Workspace,
    cx: &App,
) -> (Option<(Entity<Pane>, usize)>, Option<Entity<Pane>>) {
    let mut existing: Option<(Entity<Pane>, usize)> = None;
    let mut subagent_only: Option<Entity<Pane>> = None;
    let Some(panes) = workspace.panes_in_group(target_group_idx) else {
        return (existing, subagent_only);
    };
    for pane in panes {
        let mut count = 0usize;
        let mut all_subagent = true;
        let mut match_idx: Option<usize> = None;
        for (idx, item) in pane.read(cx).items().enumerate() {
            count += 1;
            match item.downcast::<ClaudeSubagentView>() {
                Some(view) => {
                    if match_idx.is_none() && view.read(cx).subagent_id == subagent_id {
                        match_idx = Some(idx);
                    }
                }
                None => all_subagent = false,
            }
        }
        if let Some(idx) = match_idx
            && existing.is_none()
        {
            existing = Some((pane.clone(), idx));
            // 동일 id 탭을 찾았으면 activate 용도로 즉시 쓰므로 전용 pane 은 더 볼 필요 없음.
            return (existing, subagent_only);
        }
        if subagent_only.is_none() && count > 0 && all_subagent {
            subagent_only = Some(pane.clone());
        }
    }
    (existing, subagent_only)
}
