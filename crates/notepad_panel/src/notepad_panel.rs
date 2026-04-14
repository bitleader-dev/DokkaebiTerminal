// 메모장 사이드 패널
// 오른쪽에 도킹되는 간단한 텍스트 메모장 패널

use anyhow::Result;
use editor::{
    actions::{Copy, Cut, Paste},
    Editor, EditorMode, MultiBufferOffset, SizingBehavior,
};
use gpui::{
    actions, anchored, deferred, div, px, App, AsyncWindowContext, Context, DismissEvent, Entity,
    EventEmitter, FocusHandle, Focusable, IntoElement, MouseButton, MouseDownEvent, ParentElement,
    Pixels, Point, Render, Styled, Subscription, WeakEntity, Window,
};
use i18n::t;
use language::language_settings::SoftWrap;
use serde::{Deserialize, Serialize};
use settings::{RegisterSetting, Settings, SettingsStore};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use terminal_view::{terminal_panel::TerminalPanel, TerminalView};
use ui::{prelude::*, ContextMenu, IconName, Label};
use workspace::{
    dock::{DockPosition, Panel, PanelEvent},
    Item, Workspace,
};

// 메모장 패널 설정
#[derive(Debug, Clone, PartialEq, RegisterSetting)]
pub struct NotepadPanelSettings {
    pub button: bool,
    pub dock: DockPosition,
    pub restore: bool,
    pub horizontal_scroll: bool,
}

impl Settings for NotepadPanelSettings {
    fn from_settings(content: &settings::SettingsContent) -> Self {
        let notepad_panel = content.notepad_panel.clone().unwrap();
        Self {
            button: notepad_panel.button.unwrap(),
            dock: notepad_panel.dock.unwrap().into(),
            restore: notepad_panel.restore.unwrap(),
            horizontal_scroll: notepad_panel.horizontal_scroll.unwrap(),
        }
    }
}

// 메모장 패널 토글 액션
actions!(notepad_panel, [ToggleFocus]);

/// 메모장 패널 초기화
pub fn init(cx: &mut App) {
    NotepadPanelSettings::register(cx);

    cx.observe_new(|workspace: &mut Workspace, _, _| {
        workspace.register_action(|workspace, _: &ToggleFocus, window, cx| {
            workspace.toggle_panel_focus::<NotepadPanel>(window, cx);
        });
    })
    .detach();
}

/// 메모장 데이터 저장 구조
#[derive(Serialize, Deserialize, Default)]
struct NotepadData {
    content: String,
}

/// 메모장 패널 구조체
pub struct NotepadPanel {
    /// 텍스트 편집기
    editor: Entity<Editor>,
    /// 저장 파일 경로
    save_path: PathBuf,
    /// 파일 시스템 (설정 저장용)
    fs: Arc<dyn fs::Fs>,
    /// 상위 워크스페이스 약한 참조 (컨텍스트 메뉴에서 터미널 패널 조회용)
    workspace: WeakEntity<Workspace>,
    /// 현재 표시 중인 우클릭 컨텍스트 메뉴와 표시 좌표/DismissEvent 구독.
    /// 패널 render에서 deferred anchored로 직접 그리고, 메뉴 dismiss 시 자동 해제된다.
    context_menu: Option<(Entity<ContextMenu>, Point<Pixels>, Subscription)>,
    /// 옵저버 변경 감지용 이전 설정값
    last_horizontal_scroll: bool,
}

impl NotepadPanel {
    pub fn new(
        workspace: &Workspace,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let fs = workspace.app_state().fs.clone();
        // 저장 경로: data_dir()/notepad.json
        let save_path = paths::data_dir().join("notepad.json");

        // 멀티라인 에디터 생성
        let editor = cx.new(|cx| {
            let mut editor = Editor::multi_line(window, cx);
            editor.set_placeholder_text(
                &t("notepad_panel.placeholder", cx).to_string(),
                window,
                cx,
            );
            // 메모장에서 불필요한 거터 요소 비활성화 → 라인 번호를 왼쪽에 밀착
            editor.set_show_runnables(false, cx);
            editor.set_show_code_actions(false, cx);
            editor.set_show_git_diff_gutter(false, cx);
            // 가로 스크롤 활성화 시 줄바꿈을 끄고, 비활성화 시 에디터 너비에 맞춰 줄바꿈
            let horizontal_scroll = NotepadPanelSettings::get_global(cx).horizontal_scroll;
            if horizontal_scroll {
                editor.set_soft_wrap_mode(SoftWrap::None, cx);
            } else {
                editor.set_soft_wrap_mode(SoftWrap::EditorWidth, cx);
            }
            // 에디터 설정의 scroll_beyond_last_line에 따라 overscroll 적용
            editor.set_mode(EditorMode::Full {
                scale_ui_elements_with_buffer_font_size: true,
                show_active_line_background: true,
                sizing_behavior: SizingBehavior::Default,
            });
            // 복원 설정이 켜져 있을 때만 기존 내용 로드
            let restore = NotepadPanelSettings::get_global(cx).restore;
            if restore {
                let text = Self::load_from_file(&save_path);
                if !text.is_empty() {
                    editor.set_text(text, window, cx);
                }
            }
            editor
        });

        // 설정 변경 감지 → 가로 스크롤 모드 반영 (값 변경 시에만 에디터 레이아웃 재계산)
        cx.observe_global::<SettingsStore>(|this, cx| {
            let horizontal_scroll = NotepadPanelSettings::get_global(cx).horizontal_scroll;
            if horizontal_scroll != this.last_horizontal_scroll {
                this.last_horizontal_scroll = horizontal_scroll;
                this.editor.update(cx, |editor, cx| {
                    if horizontal_scroll {
                        editor.set_soft_wrap_mode(SoftWrap::None, cx);
                    } else {
                        editor.set_soft_wrap_mode(SoftWrap::EditorWidth, cx);
                    }
                });
            }
        })
        .detach();

        // 에디터 변경 감지 → 자동 저장
        cx.subscribe_in(&editor, window, |this, _editor, event: &editor::EditorEvent, _window, cx| {
            if matches!(event, editor::EditorEvent::BufferEdited { .. }) {
                this.save_content(cx);
            }
        })
        .detach();

        Self {
            editor,
            save_path,
            fs,
            workspace: workspace.weak_handle(),
            context_menu: None,
            last_horizontal_scroll: NotepadPanelSettings::get_global(cx).horizontal_scroll,
        }
    }

    /// 파일에서 메모 내용 로드
    fn load_from_file(path: &PathBuf) -> String {
        if let Ok(data) = std::fs::read_to_string(path) {
            if let Ok(notepad_data) = serde_json::from_str::<NotepadData>(&data) {
                return notepad_data.content;
            }
        }
        String::new()
    }

    /// 현재 에디터 내용을 파일에 저장
    fn save_content(&self, cx: &App) {
        let text = self.editor.read(cx).text(cx);
        // 마지막 빈 라인 제거 후 저장
        let trimmed = text.trim_end_matches(|c: char| c == '\n' || c == '\r');
        let data = NotepadData { content: trimmed.to_string() };
        if let Some(parent) = self.save_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(&data) {
            let _ = std::fs::write(&self.save_path, json);
        }
    }

    /// 비동기 로드
    pub async fn load(
        workspace: WeakEntity<Workspace>,
        mut cx: AsyncWindowContext,
    ) -> Result<Entity<Self>> {
        workspace.update_in(&mut cx, |workspace, window, cx| {
            cx.new(|cx| NotepadPanel::new(workspace, window, cx))
        })
    }

    /// 에디터의 현재 선택 텍스트를 결합해 반환. 공백뿐이거나 선택이 없으면 None.
    /// 다중 커서 선택은 개행으로 이어 붙인다.
    fn selected_text(editor: &Entity<Editor>, cx: &mut App) -> Option<String> {
        editor.update(cx, |editor, cx| {
            let snapshot = editor.display_snapshot(cx);
            let selections = editor.selections.all::<MultiBufferOffset>(&snapshot);
            let buffer = editor.buffer().read(cx).read(cx);
            let mut parts: Vec<String> = Vec::new();
            for selection in selections.iter() {
                if selection.start == selection.end {
                    continue;
                }
                let text: String = buffer
                    .text_for_range(selection.start..selection.end)
                    .collect();
                if !text.is_empty() {
                    parts.push(text);
                }
            }
            if parts.is_empty() {
                return None;
            }
            let joined = parts.join("\n");
            if joined.trim().is_empty() {
                None
            } else {
                Some(joined)
            }
        })
    }

    /// 워크스페이스에 존재하는 모든 터미널 탭을 수집.
    /// - Dock의 TerminalPanel 내부 pane들
    /// - 중앙 pane에 열린 터미널 (workspace.items_of_type)
    /// 반환 튜플은 (메뉴에 표시할 라벨, TerminalView 엔티티). 동일 라벨이 중복되면 `(N)` 접미사 부여.
    fn collect_terminals(
        workspace: &Workspace,
        cx: &App,
    ) -> Vec<(SharedString, Entity<TerminalView>)> {
        let mut base_names: Vec<(SharedString, Entity<TerminalView>)> = Vec::new();
        let mut seen: std::collections::HashSet<gpui::EntityId> = std::collections::HashSet::new();

        // 1) TerminalPanel (dock) 내부 pane 순회
        if let Some(terminal_panel) = workspace.panel::<TerminalPanel>(cx) {
            let terminal_panel = terminal_panel.read(cx);
            for pane in terminal_panel.panes() {
                for item in pane.read(cx).items() {
                    let Some(terminal_view) = item.downcast::<TerminalView>() else {
                        continue;
                    };
                    if !seen.insert(terminal_view.entity_id()) {
                        continue;
                    }
                    let label = terminal_view.read(cx).tab_content_text(0, cx);
                    base_names.push((label, terminal_view));
                }
            }
        }

        // 2) 중앙 pane에 열린 TerminalView 순회
        for terminal_view in workspace.items_of_type::<TerminalView>(cx) {
            if !seen.insert(terminal_view.entity_id()) {
                continue;
            }
            let label = terminal_view.read(cx).tab_content_text(0, cx);
            base_names.push((label, terminal_view));
        }

        // 같은 라벨이 여러 개면 등장 순서대로 `(2)`, `(3)` 접미사를 붙여 구분한다.
        let mut total_counts: HashMap<SharedString, u32> = HashMap::new();
        for (label, _) in base_names.iter() {
            *total_counts.entry(label.clone()).or_insert(0) += 1;
        }
        let mut running_counts: HashMap<SharedString, u32> = HashMap::new();
        let mut result: Vec<(SharedString, Entity<TerminalView>)> = Vec::with_capacity(base_names.len());
        for (label, view) in base_names.into_iter() {
            let total = total_counts.get(&label).copied().unwrap_or(1);
            if total <= 1 {
                result.push((label, view));
            } else {
                let n = running_counts.entry(label.clone()).or_insert(0);
                *n += 1;
                let numbered = SharedString::from(format!("{} ({})", label, *n));
                result.push((numbered, view));
            }
        }
        result
    }

    /// 메모장 전용 우클릭 컨텍스트 메뉴를 구성해 NotepadPanel의 `context_menu` 필드에 저장한다.
    /// render에서 deferred anchored로 그려진다.
    /// - 선택 텍스트가 공백뿐이거나 비어있으면 메뉴를 띄우지 않는다.
    /// - 현재 워크스페이스의 TerminalPanel에 있는 모든 터미널 탭을 항목으로 노출한다.
    /// - 터미널 탭이 하나도 없으면 메뉴를 띄우지 않는다.
    ///
    /// 메뉴를 실제로 띄웠으면 `true`를 반환한다. 호출 측은 이 값으로 이벤트 전파 차단 여부를 결정한다.
    fn deploy_terminal_send_menu(
        &mut self,
        position: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let editor = self.editor.clone();
        // 선택 텍스트는 터미널 섹션용. 없어도 편집 섹션은 표시하므로 여기서 return하지 않는다.
        let selected_text = Self::selected_text(&editor, cx);

        let terminals = self
            .workspace
            .upgrade()
            .map(|workspace| {
                workspace.read_with(cx, |workspace, cx| Self::collect_terminals(workspace, cx))
            })
            .unwrap_or_default();

        // 메뉴 라벨 준비 (i18n): 편집 메뉴 3종 + 터미널 입력 접미사
        let copy_label: SharedString = t("notepad_panel.context_menu.copy", cx).into();
        let cut_label: SharedString = t("notepad_panel.context_menu.cut", cx).into();
        let paste_label: SharedString = t("notepad_panel.context_menu.paste", cx).into();
        let send_suffix = t("notepad_panel.context_menu.send_suffix", cx).to_string();

        // action의 키 바인딩이 Editor 키맵 컨텍스트에서 조회되도록 editor의 focus handle을 action_context로 지정.
        let editor_focus = editor.focus_handle(cx);
        let editor_for_menu = editor.clone();
        let menu = ContextMenu::build(window, cx, move |mut menu, _window, _cx| {
            menu = menu.context(editor_focus);
            // 편집 섹션: 복사 / 잘라내기 / 붙여넣기 (action 바인딩으로 단축키 표시)
            menu = menu.entry(copy_label, Some(Box::new(Copy)), {
                let editor = editor_for_menu.clone();
                move |window, cx| {
                    editor.update(cx, |editor, cx| editor.copy(&Copy, window, cx));
                }
            });
            menu = menu.entry(cut_label, Some(Box::new(Cut)), {
                let editor = editor_for_menu.clone();
                move |window, cx| {
                    editor.update(cx, |editor, cx| editor.cut(&Cut, window, cx));
                }
            });
            menu = menu.entry(paste_label, Some(Box::new(Paste)), {
                let editor = editor_for_menu.clone();
                move |window, cx| {
                    editor.update(cx, |editor, cx| editor.paste(&Paste, window, cx));
                }
            });

            // 터미널 섹션: 선택 텍스트가 있고 터미널 탭이 1개 이상일 때만 구분선 + 엔트리 추가.
            if let Some(text) = selected_text.as_ref() {
                if !terminals.is_empty() {
                    menu = menu.separator();
                    for (label, terminal_view) in terminals.iter().cloned() {
                        let text = text.clone();
                        let entry_label =
                            SharedString::from(format!("{} {}", label, send_suffix));
                        menu = menu.entry(entry_label, None, move |_window, cx| {
                            terminal_view.update(cx, |view, cx| {
                                view.terminal().update(cx, |terminal, _cx| {
                                    terminal.paste(&text);
                                });
                            });
                        });
                    }
                }
            }
            menu
        });

        // 메뉴에 포커스 이동 → blur 시 dismiss.
        window.focus(&menu.focus_handle(cx), cx);
        // 메뉴 dismiss 이벤트 구독 → 상태 정리.
        let subscription = cx.subscribe(&menu, |this, _, _: &DismissEvent, cx| {
            this.context_menu.take();
            cx.notify();
        });
        self.context_menu = Some((menu, position, subscription));
        cx.notify();
        true
    }
}

impl Focusable for NotepadPanel {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.editor.focus_handle(cx)
    }
}

impl EventEmitter<PanelEvent> for NotepadPanel {}

impl Render for NotepadPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("notepad-panel")
            .key_context("NotepadPanel")
            .track_focus(&self.editor.focus_handle(cx))
            .size_full()
            .flex()
            .flex_col()
            .bg(cx.theme().colors().panel_background)
            // 헤더
            .child(
                div()
                    .flex()
                    .items_center()
                    .px_2()
                    .py_1()
                    .border_b_1()
                    .border_color(cx.theme().colors().border)
                    .child(
                        Label::new(t("notepad_panel.title", cx))
                            .size(LabelSize::Small)
                            .color(Color::Default),
                    ),
            )
            // 에디터 영역 — 우클릭은 capture phase에서 가로채어 터미널 전송 메뉴를 띄운다.
            // Editor가 bubble phase에서 자체 컨텍스트 메뉴를 deploy하므로 capture로 선점하고,
            // 메뉴를 실제로 띄운 경우에만 전파를 차단한다.
            // 메뉴는 NotepadPanel이 직접 `context_menu` 필드에 소유하며 deferred anchored로 그린다.
            .child(
                div()
                    .id("notepad-editor-area")
                    .flex_1()
                    .size_full()
                    .occlude()
                    .capture_any_mouse_down(cx.listener(
                        |this, event: &MouseDownEvent, window, cx| {
                            if event.button != MouseButton::Right {
                                return;
                            }
                            if this.deploy_terminal_send_menu(event.position, window, cx) {
                                cx.stop_propagation();
                            }
                        },
                    ))
                    .child(self.editor.clone()),
            )
            .children(self.context_menu.as_ref().map(|(menu, position, _)| {
                deferred(
                    anchored()
                        .position(*position)
                        .anchor(gpui::Corner::TopLeft)
                        .child(menu.clone()),
                )
                .with_priority(1)
            }))
    }
}

impl Panel for NotepadPanel {
    fn persistent_name() -> &'static str {
        "Notepad Panel"
    }

    fn panel_key() -> &'static str {
        "NotepadPanel"
    }

    fn position(&self, _window: &Window, cx: &App) -> DockPosition {
        NotepadPanelSettings::get_global(cx).dock
    }

    fn position_is_valid(&self, position: DockPosition) -> bool {
        matches!(
            position,
            DockPosition::Left | DockPosition::Bottom | DockPosition::Right
        )
    }

    fn set_position(
        &mut self,
        position: DockPosition,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        settings::update_settings_file(self.fs.clone(), cx, move |settings, _| {
            settings
                .notepad_panel
                .get_or_insert_default()
                .dock = Some(position.into());
        });
    }

    fn default_size(&self, _window: &Window, _cx: &App) -> Pixels {
        px(300.)
    }

    fn icon(&self, _window: &Window, cx: &App) -> Option<IconName> {
        Some(IconName::Notepad).filter(|_| NotepadPanelSettings::get_global(cx).button)
    }

    fn icon_tooltip(&self, _window: &Window, _cx: &App) -> Option<&'static str> {
        Some("notepad_panel.tooltip")
    }

    fn toggle_action(&self) -> Box<dyn gpui::Action> {
        Box::new(ToggleFocus)
    }

    fn activation_priority(&self) -> u32 {
        9
    }
}
