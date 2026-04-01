// 메모장 사이드 패널
// 오른쪽에 도킹되는 간단한 텍스트 메모장 패널

use anyhow::Result;
use editor::Editor;
use gpui::{
    actions, div, px, App, AsyncWindowContext, Context, Entity, EventEmitter, FocusHandle,
    Focusable, IntoElement, ParentElement, Pixels, Render, Styled, WeakEntity, Window,
};
use i18n::t;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use ui::{prelude::*, IconName, Label};
use workspace::{
    dock::{DockPosition, Panel, PanelEvent},
    Workspace,
};

// 메모장 패널 토글 액션
actions!(notepad_panel, [ToggleFocus]);

/// 메모장 패널 초기화
pub fn init(cx: &mut App) {
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
}

impl NotepadPanel {
    pub fn new(
        _workspace: &Workspace,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
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
            // 기존 내용 로드
            let text = Self::load_from_file(&save_path);
            if !text.is_empty() {
                editor.set_text(text, window, cx);
            }
            editor
        });

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
        }
    }

    /// 파일에서 메모 내용 로드
    fn load_from_file(path: &PathBuf) -> String {
        if let Ok(data) = fs::read_to_string(path) {
            if let Ok(notepad_data) = serde_json::from_str::<NotepadData>(&data) {
                return notepad_data.content;
            }
        }
        String::new()
    }

    /// 현재 에디터 내용을 파일에 저장
    fn save_content(&self, cx: &App) {
        let text = self.editor.read(cx).text(cx);
        let data = NotepadData { content: text };
        if let Some(parent) = self.save_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(&data) {
            let _ = fs::write(&self.save_path, json);
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
            // 에디터 영역
            .child(
                div()
                    .flex_1()
                    .size_full()
                    .child(self.editor.clone()),
            )
    }
}

impl Panel for NotepadPanel {
    fn persistent_name() -> &'static str {
        "Notepad Panel"
    }

    fn panel_key() -> &'static str {
        "NotepadPanel"
    }

    fn position(&self, _window: &Window, _cx: &App) -> DockPosition {
        DockPosition::Right
    }

    fn position_is_valid(&self, position: DockPosition) -> bool {
        matches!(position, DockPosition::Left | DockPosition::Right)
    }

    fn set_position(
        &mut self,
        _position: DockPosition,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
    }

    fn default_size(&self, _window: &Window, _cx: &App) -> Pixels {
        px(300.)
    }

    fn icon(&self, _window: &Window, _cx: &App) -> Option<IconName> {
        Some(IconName::Notepad)
    }

    fn icon_tooltip(&self, _window: &Window, _cx: &App) -> Option<&'static str> {
        Some("Notepad Panel")
    }

    fn toggle_action(&self) -> Box<dyn gpui::Action> {
        Box::new(ToggleFocus)
    }

    fn activation_priority(&self) -> u32 {
        8
    }
}
