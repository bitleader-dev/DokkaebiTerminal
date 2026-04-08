// 워크스페이스 그룹 패널 — 좌측 독에 표시되는 워크스페이스 그룹 목록 관리 UI

use std::ops::Range;

use gpui::{
    Action, App, Bounds, Context, DismissEvent, ElementId, ElementInputHandler, Entity,
    EntityInputHandler, EventEmitter, FocusHandle, Focusable, GlobalElementId, InteractiveElement,
    IntoElement, LayoutId, MouseButton, ParentElement, PaintQuad, Pixels, Point, Render,
    SharedString, Style, Styled, TextRun, UTF16Selection, WeakEntity, Window, actions, fill, point,
    px, relative, size,
};
use ui::{
    Clickable, ContextMenu, FluentBuilder, IconButton, IconName, Tooltip,
    prelude::*,
};
use i18n::t;
use unicode_segmentation::UnicodeSegmentation;

use crate::{
    Workspace,
    dock::{DockPosition, Panel, PanelEvent},
};

use gpui::AsyncWindowContext;

const WORKSPACE_GROUP_PANEL_KEY: &str = "WorkspaceGroupPanel";

actions!(
    workspace_group_panel,
    [
        /// 워크스페이스 그룹 패널 토글
        ToggleWorkspaceGroupPanel,
    ]
);

/// 워크스페이스 그룹 패널을 Workspace에 등록 (현재 미사용 — action은 new()에서 직접 등록)
pub fn init(_cx: &mut App) {
    // action 등록은 new()에서 수행
}

// ── 인라인 이름 편집기 ──────────────────────────────────────────────

/// 인라인 텍스트 편집기 — 워크스페이스 그룹 이름 변경 시 사용
struct RenameEditor {
    focus_handle: FocusHandle,
    content: SharedString,
    selected_range: Range<usize>,
    selection_reversed: bool,
    marked_range: Option<Range<usize>>,
    last_layout: Option<gpui::ShapedLine>,
    last_bounds: Option<Bounds<Pixels>>,
}

// 인라인 텍스트 편집 액션
actions!(
    rename_editor,
    [
        RenameBackspace,
        RenameDelete,
        RenameLeft,
        RenameRight,
        RenameSelectAll,
        RenameHome,
        RenameEnd,
        RenamePaste,
        RenameCopy,
        RenameCut,
    ]
);

impl RenameEditor {
    fn new(initial_text: String, cx: &mut Context<Self>) -> Self {
        let len = initial_text.len();
        Self {
            focus_handle: cx.focus_handle(),
            content: initial_text.into(),
            selected_range: 0..len, // 전체 선택
            selection_reversed: false,
            marked_range: None,
            last_layout: None,
            last_bounds: None,
        }
    }

    fn text(&self) -> String {
        self.content.to_string()
    }

    fn backspace(&mut self, _: &RenameBackspace, window: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            self.select_to(self.previous_boundary(self.cursor_offset()), cx)
        }
        self.replace_text_in_range(None, "", window, cx)
    }

    fn delete(&mut self, _: &RenameDelete, window: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            self.select_to(self.next_boundary(self.cursor_offset()), cx)
        }
        self.replace_text_in_range(None, "", window, cx)
    }

    fn left(&mut self, _: &RenameLeft, _: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            self.move_to(self.previous_boundary(self.cursor_offset()), cx);
        } else {
            self.move_to(self.selected_range.start, cx)
        }
    }

    fn right(&mut self, _: &RenameRight, _: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            self.move_to(self.next_boundary(self.selected_range.end), cx);
        } else {
            self.move_to(self.selected_range.end, cx)
        }
    }

    fn select_all(&mut self, _: &RenameSelectAll, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(0, cx);
        self.select_to(self.content.len(), cx)
    }

    fn home(&mut self, _: &RenameHome, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(0, cx);
    }

    fn end(&mut self, _: &RenameEnd, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(self.content.len(), cx);
    }

    fn paste(&mut self, _: &RenamePaste, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
            self.replace_text_in_range(None, &text.replace("\n", ""), window, cx);
        }
    }

    fn copy(&mut self, _: &RenameCopy, _: &mut Window, cx: &mut Context<Self>) {
        if !self.selected_range.is_empty() {
            cx.write_to_clipboard(gpui::ClipboardItem::new_string(
                self.content[self.selected_range.clone()].to_string(),
            ));
        }
    }

    fn cut(&mut self, _: &RenameCut, window: &mut Window, cx: &mut Context<Self>) {
        if !self.selected_range.is_empty() {
            cx.write_to_clipboard(gpui::ClipboardItem::new_string(
                self.content[self.selected_range.clone()].to_string(),
            ));
            self.replace_text_in_range(None, "", window, cx)
        }
    }

    fn move_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        self.selected_range = offset..offset;
        cx.notify()
    }

    fn cursor_offset(&self) -> usize {
        if self.selection_reversed {
            self.selected_range.start
        } else {
            self.selected_range.end
        }
    }

    fn select_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        if self.selection_reversed {
            self.selected_range.start = offset
        } else {
            self.selected_range.end = offset
        };
        if self.selected_range.end < self.selected_range.start {
            self.selection_reversed = !self.selection_reversed;
            self.selected_range = self.selected_range.end..self.selected_range.start;
        }
        cx.notify()
    }

    fn previous_boundary(&self, offset: usize) -> usize {
        self.content
            .grapheme_indices(true)
            .rev()
            .find_map(|(idx, _)| (idx < offset).then_some(idx))
            .unwrap_or(0)
    }

    fn next_boundary(&self, offset: usize) -> usize {
        self.content
            .grapheme_indices(true)
            .find_map(|(idx, _)| (idx > offset).then_some(idx))
            .unwrap_or(self.content.len())
    }

    // ── UTF-8 ↔ UTF-16 변환 ──

    fn offset_from_utf16(&self, offset: usize) -> usize {
        let mut utf8_offset = 0;
        let mut utf16_count = 0;
        for ch in self.content.chars() {
            if utf16_count >= offset {
                break;
            }
            utf16_count += ch.len_utf16();
            utf8_offset += ch.len_utf8();
        }
        utf8_offset
    }

    fn offset_to_utf16(&self, offset: usize) -> usize {
        let mut utf16_offset = 0;
        let mut utf8_count = 0;
        for ch in self.content.chars() {
            if utf8_count >= offset {
                break;
            }
            utf8_count += ch.len_utf8();
            utf16_offset += ch.len_utf16();
        }
        utf16_offset
    }

    fn range_to_utf16(&self, range: &Range<usize>) -> Range<usize> {
        self.offset_to_utf16(range.start)..self.offset_to_utf16(range.end)
    }

    fn range_from_utf16(&self, range_utf16: &Range<usize>) -> Range<usize> {
        self.offset_from_utf16(range_utf16.start)..self.offset_from_utf16(range_utf16.end)
    }
}

impl Focusable for RenameEditor {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl EntityInputHandler for RenameEditor {
    fn text_for_range(
        &mut self,
        range_utf16: Range<usize>,
        actual_range: &mut Option<Range<usize>>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<String> {
        let range = self.range_from_utf16(&range_utf16);
        actual_range.replace(self.range_to_utf16(&range));
        Some(self.content[range].to_string())
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        Some(UTF16Selection {
            range: self.range_to_utf16(&self.selected_range),
            reversed: self.selection_reversed,
        })
    }

    fn marked_text_range(
        &self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Range<usize>> {
        self.marked_range
            .as_ref()
            .map(|range| self.range_to_utf16(range))
    }

    fn unmark_text(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {
        self.marked_range = None;
    }

    fn replace_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = range_utf16
            .as_ref()
            .map(|r| self.range_from_utf16(r))
            .or(self.marked_range.clone())
            .unwrap_or(self.selected_range.clone());

        self.content =
            (self.content[0..range.start].to_owned() + new_text + &self.content[range.end..])
                .into();
        self.selected_range = range.start + new_text.len()..range.start + new_text.len();
        self.marked_range.take();
        cx.notify();
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        new_selected_range_utf16: Option<Range<usize>>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = range_utf16
            .as_ref()
            .map(|r| self.range_from_utf16(r))
            .or(self.marked_range.clone())
            .unwrap_or(self.selected_range.clone());

        self.content =
            (self.content[0..range.start].to_owned() + new_text + &self.content[range.end..])
                .into();
        if !new_text.is_empty() {
            self.marked_range = Some(range.start..range.start + new_text.len());
        } else {
            self.marked_range = None;
        }
        self.selected_range = new_selected_range_utf16
            .as_ref()
            .map(|r| self.range_from_utf16(r))
            .map(|new_range| new_range.start + range.start..new_range.end + range.end)
            .unwrap_or_else(|| range.start + new_text.len()..range.start + new_text.len());
        cx.notify();
    }

    fn bounds_for_range(
        &mut self,
        range_utf16: Range<usize>,
        bounds: Bounds<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        let last_layout = self.last_layout.as_ref()?;
        let range = self.range_from_utf16(&range_utf16);
        Some(Bounds::from_corners(
            point(
                bounds.left() + last_layout.x_for_index(range.start),
                bounds.top(),
            ),
            point(
                bounds.left() + last_layout.x_for_index(range.end),
                bounds.bottom(),
            ),
        ))
    }

    fn character_index_for_point(
        &mut self,
        point: gpui::Point<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<usize> {
        let line_point = self.last_bounds?.localize(&point)?;
        let last_layout = self.last_layout.as_ref()?;
        let utf8_index = last_layout.index_for_x(point.x - line_point.x)?;
        Some(self.offset_to_utf16(utf8_index))
    }
}

impl Render for RenameEditor {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .key_context("RenameEditor")
            .track_focus(&self.focus_handle)
            .cursor(gpui::CursorStyle::IBeam)
            .on_action(cx.listener(Self::backspace))
            .on_action(cx.listener(Self::delete))
            .on_action(cx.listener(Self::left))
            .on_action(cx.listener(Self::right))
            .on_action(cx.listener(Self::select_all))
            .on_action(cx.listener(Self::home))
            .on_action(cx.listener(Self::end))
            .on_action(cx.listener(Self::paste))
            .on_action(cx.listener(Self::copy))
            .on_action(cx.listener(Self::cut))
            .size_full()
            .child(RenameEditorElement {
                editor: cx.entity(),
            })
    }
}

/// 인라인 편집기 렌더링 Element — window.handle_input() 연결
struct RenameEditorElement {
    editor: Entity<RenameEditor>,
}

impl IntoElement for RenameEditorElement {
    type Element = Self;
    fn into_element(self) -> Self::Element {
        self
    }
}

struct RenameEditorPrepaint {
    line: Option<gpui::ShapedLine>,
    cursor: Option<PaintQuad>,
    selection: Option<PaintQuad>,
}

impl Element for RenameEditorElement {
    type RequestLayoutState = ();
    type PrepaintState = RenameEditorPrepaint;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let mut style = Style::default();
        style.size.width = relative(1.).into();
        style.size.height = window.line_height().into();
        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let editor = self.editor.read(cx);
        let content = editor.content.clone();
        let selected_range = editor.selected_range.clone();
        let cursor = editor.cursor_offset();
        let style = window.text_style();

        let display_text = if content.is_empty() {
            SharedString::from("")
        } else {
            content
        };

        let text_color = style.color;
        let run = TextRun {
            len: display_text.len(),
            font: style.font(),
            color: text_color,
            background_color: None,
            underline: None,
            strikethrough: None,
        };

        let runs = if let Some(marked_range) = editor.marked_range.as_ref() {
            vec![
                TextRun {
                    len: marked_range.start,
                    ..run.clone()
                },
                TextRun {
                    len: marked_range.end - marked_range.start,
                    underline: Some(gpui::UnderlineStyle {
                        color: Some(run.color),
                        thickness: px(1.0),
                        wavy: false,
                    }),
                    ..run.clone()
                },
                TextRun {
                    len: display_text.len() - marked_range.end,
                    ..run
                },
            ]
            .into_iter()
            .filter(|r| r.len > 0)
            .collect()
        } else {
            vec![run]
        };

        let font_size = style.font_size.to_pixels(window.rem_size());
        let line = window
            .text_system()
            .shape_line(display_text, font_size, &runs, None);

        let cursor_pos = line.x_for_index(cursor);
        let (selection, cursor_quad) = if selected_range.is_empty() {
            (
                None,
                Some(fill(
                    Bounds::new(
                        point(bounds.left() + cursor_pos, bounds.top()),
                        size(px(1.), bounds.bottom() - bounds.top()),
                    ),
                    text_color,
                )),
            )
        } else {
            (
                Some(fill(
                    Bounds::from_corners(
                        point(
                            bounds.left() + line.x_for_index(selected_range.start),
                            bounds.top(),
                        ),
                        point(
                            bounds.left() + line.x_for_index(selected_range.end),
                            bounds.bottom(),
                        ),
                    ),
                    gpui::rgba(0x3388ff40),
                )),
                None,
            )
        };

        RenameEditorPrepaint {
            line: Some(line),
            cursor: cursor_quad,
            selection,
        }
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let focus_handle = self.editor.read(cx).focus_handle.clone();
        window.handle_input(
            &focus_handle,
            ElementInputHandler::new(bounds, self.editor.clone()),
            cx,
        );
        if let Some(selection) = prepaint.selection.take() {
            window.paint_quad(selection)
        }
        let line = prepaint.line.take().unwrap();
        line.paint(
            bounds.origin,
            window.line_height(),
            gpui::TextAlign::Left,
            None,
            window,
            cx,
        )
        .unwrap();

        if focus_handle.is_focused(window) {
            if let Some(cursor) = prepaint.cursor.take() {
                window.paint_quad(cursor);
            }
        }

        self.editor.update(cx, |editor, _cx| {
            editor.last_layout = Some(line);
            editor.last_bounds = Some(bounds);
        });
    }
}

// ── 드래그 앤 드롭 ────────────────────────────────────────────────

/// 워크스페이스 그룹 드래그 데이터
#[derive(Clone)]
struct DraggedWorkspaceGroup {
    index: usize,
    name: SharedString,
}

impl Render for DraggedWorkspaceGroup {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = cx.theme().colors();
        div()
            .px_2()
            .py(px(4.))
            .rounded_md()
            .bg(colors.element_selected)
            .border_1()
            .border_color(colors.border_focused)
            .text_sm()
            .text_color(colors.text)
            .opacity(0.85)
            .child(self.name.clone())
    }
}

// ── 워크스페이스 그룹 패널 ──────────────────────────────────────────

pub struct WorkspaceGroupPanel {
    workspace: WeakEntity<Workspace>,
    focus_handle: FocusHandle,
    /// 우클릭 컨텍스트 메뉴
    context_menu: Option<(Entity<ContextMenu>, Point<Pixels>, gpui::Subscription)>,
    /// 이름 편집 중인 그룹 인덱스와 편집기
    editing: Option<(usize, Entity<RenameEditor>)>,
    /// 중복 이름 에러 표시 여부
    rename_error: bool,
}

impl WorkspaceGroupPanel {
    /// ProjectPanel::new()와 동일한 시그니처
    fn new(
        workspace: &mut Workspace,
        _window: &mut Window,
        cx: &mut Context<Workspace>,
    ) -> Entity<Self> {
        // action handler를 workspace에 직접 등록
        workspace.register_action(|workspace, _: &ToggleWorkspaceGroupPanel, window, cx| {
            workspace.toggle_panel_focus::<WorkspaceGroupPanel>(window, cx);
        });

        let workspace_handle = workspace.weak_handle();
        let workspace_entity = cx.entity().clone();
        cx.new(|cx| {
            // 워크스페이스 상태 변경 시 패널 다시 렌더링
            cx.observe(&workspace_entity, |_this, _workspace, cx| {
                cx.notify();
            })
            .detach();

            // 인라인 편집기 키바인딩 등록
            cx.bind_keys([
                gpui::KeyBinding::new("backspace", RenameBackspace, Some("RenameEditor")),
                gpui::KeyBinding::new("delete", RenameDelete, Some("RenameEditor")),
                gpui::KeyBinding::new("left", RenameLeft, Some("RenameEditor")),
                gpui::KeyBinding::new("right", RenameRight, Some("RenameEditor")),
                gpui::KeyBinding::new("ctrl-a", RenameSelectAll, Some("RenameEditor")),
                gpui::KeyBinding::new("home", RenameHome, Some("RenameEditor")),
                gpui::KeyBinding::new("end", RenameEnd, Some("RenameEditor")),
                gpui::KeyBinding::new("ctrl-v", RenamePaste, Some("RenameEditor")),
                gpui::KeyBinding::new("ctrl-c", RenameCopy, Some("RenameEditor")),
                gpui::KeyBinding::new("ctrl-x", RenameCut, Some("RenameEditor")),
            ]);

            Self {
                workspace: workspace_handle,
                focus_handle: cx.focus_handle(),
                context_menu: None,
                editing: None,
                rename_error: false,
            }
        })
    }

    /// ProjectPanel::load()와 동일한 패턴
    pub async fn load(
        workspace: WeakEntity<Workspace>,
        mut cx: AsyncWindowContext,
    ) -> anyhow::Result<Entity<Self>> {
        workspace.update_in(&mut cx, |workspace, window, cx| {
            WorkspaceGroupPanel::new(workspace, window, cx)
        })
    }

    /// 워크스페이스 그룹 추가
    fn add_group(&self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(workspace) = self.workspace.upgrade() {
            workspace.update(cx, |workspace, cx| {
                workspace.add_workspace_group(window, cx);
            });
            cx.notify();
        }
    }

    /// 워크스페이스 그룹 전환
    fn switch_group(&self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(workspace) = self.workspace.upgrade() {
            workspace.update(cx, |workspace, cx| {
                workspace.switch_workspace_group(index, window, cx);
            });
            cx.notify();
        }
    }

    /// 워크스페이스 그룹 삭제
    fn remove_group(&self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(workspace) = self.workspace.upgrade() {
            workspace.update(cx, |workspace, cx| {
                workspace.remove_workspace_group(index, window, cx);
            });
            cx.notify();
        }
    }

    /// 워크스페이스 그룹 순서 이동
    fn move_group(&self, from: usize, to: usize, window: &mut Window, cx: &mut Context<Self>) {
        if from == to {
            return;
        }
        if let Some(workspace) = self.workspace.upgrade() {
            workspace.update(cx, |workspace, cx| {
                workspace.move_workspace_group(from, to, window, cx);
            });
            cx.notify();
        }
    }

    /// 이름 편집 시작
    fn start_rename(&mut self, index: usize, name: String, window: &mut Window, cx: &mut Context<Self>) {
        self.rename_error = false;
        let editor = cx.new(|cx| RenameEditor::new(name, cx));

        // 편집기 포커스 해제 시 확정
        let editor_focus = editor.focus_handle(cx);
        cx.on_blur(&editor_focus, window, |this, window, cx| {
            if this.editing.is_some() {
                this.confirm_rename(window, cx);
            }
        }).detach();

        window.focus(&editor.focus_handle(cx), cx);
        self.editing = Some((index, editor));
        cx.notify();
    }

    /// 이름 편집 확정
    fn confirm_rename(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some((index, editor)) = self.editing.take() else {
            return;
        };
        let new_name = editor.read(cx).text().trim().to_string();

        if new_name.is_empty() {
            // 빈 이름이면 취소
            self.rename_error = false;
            cx.notify();
            return;
        }

        if let Some(workspace) = self.workspace.upgrade() {
            let success = workspace.update(cx, |workspace, cx| {
                workspace.rename_workspace_group(index, new_name.clone(), window, cx)
            });

            if !success {
                // 중복 이름 — 편집기 유지
                self.editing = Some((index, editor));
                self.rename_error = true;
                cx.notify();
                return;
            }
        }

        self.rename_error = false;
        cx.notify();
    }

    /// 이름 편집 취소
    fn cancel_rename(&mut self, cx: &mut Context<Self>) {
        self.editing = None;
        self.rename_error = false;
        cx.notify();
    }

    /// 우클릭 컨텍스트 메뉴 표시
    fn deploy_context_menu(
        &mut self,
        position: Point<Pixels>,
        index: usize,
        name: String,
        group_count: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let can_delete = group_count > 1;
        let entity = cx.entity();

        let context_menu = ContextMenu::build(window, cx, move |menu, _, cx| {
            let label_rename = t("workspace_group.menu.rename", cx);
            let label_delete = t("workspace_group.menu.delete", cx);
            menu.entry(label_rename, None, {
                let entity = entity.clone();
                let name = name.clone();
                move |window, cx| {
                    entity.update(cx, |this, cx| {
                        this.start_rename(index, name.clone(), window, cx);
                    });
                }
            })
            .when(can_delete, |menu| {
                menu.separator()
                    .entry(label_delete, None, {
                        let entity = entity.clone();
                        move |window, cx| {
                            entity.update(cx, |this, cx| {
                                this.remove_group(index, window, cx);
                            });
                        }
                    })
            })
        });

        window.focus(&context_menu.focus_handle(cx), cx);
        let subscription = cx.subscribe(&context_menu, |this, _, _: &DismissEvent, cx| {
            this.context_menu.take();
            cx.notify();
        });
        self.context_menu = Some((context_menu, position, subscription));
        cx.notify();
    }
}

impl EventEmitter<PanelEvent> for WorkspaceGroupPanel {}

impl Focusable for WorkspaceGroupPanel {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Panel for WorkspaceGroupPanel {
    fn position(&self, _window: &Window, _cx: &App) -> DockPosition {
        DockPosition::Left
    }

    fn position_is_valid(&self, position: DockPosition) -> bool {
        matches!(position, DockPosition::Left)
    }

    fn set_position(
        &mut self,
        _position: DockPosition,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
        // 고정 위치 — 항상 왼쪽
    }

    fn default_size(&self, _window: &Window, _cx: &App) -> gpui::Pixels {
        px(200.)
    }

    fn icon(&self, _window: &Window, _cx: &App) -> Option<IconName> {
        // dock 버튼 목록에는 표시하지 않음 (타이틀바에서 토글)
        None
    }

    fn icon_tooltip(&self, _window: &Window, _cx: &App) -> Option<&'static str> {
        Some("workspace_group.panel.tooltip")
    }

    fn toggle_action(&self) -> Box<dyn Action> {
        Box::new(ToggleWorkspaceGroupPanel)
    }

    fn persistent_name() -> &'static str {
        "WorkspaceGroupPanel"
    }

    fn panel_key() -> &'static str {
        WORKSPACE_GROUP_PANEL_KEY
    }

    fn starts_open(&self, _window: &Window, _cx: &App) -> bool {
        false
    }

    fn activation_priority(&self) -> u32 {
        8
    }
}

impl Render for WorkspaceGroupPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (groups, active_index, group_count) =
            if let Some(workspace) = self.workspace.upgrade() {
                let ws = workspace.read(cx);
                let groups: Vec<(usize, String, bool)> = ws
                    .workspace_groups()
                    .iter()
                    .enumerate()
                    .map(|(i, g)| (i, g.name.clone(), g.has_notification))
                    .collect();
                let active = ws.active_group_index();
                let count = ws.workspace_group_count();
                (groups, active, count)
            } else {
                (Vec::new(), 0, 0)
            };

        let editing_index = self.editing.as_ref().map(|(idx, _)| *idx);
        let rename_error = self.rename_error;

        // i18n 문자열
        let label_title = t("workspace_group.panel.title", cx);
        let label_add = t("workspace_group.add_tooltip", cx);
        let label_delete_tooltip = t("workspace_group.delete_tooltip", cx);
        let label_rename_error = t("workspace_group.rename_error.duplicate", cx);

        let colors = cx.theme().colors();

        v_flex()
            .id("workspace-group-panel")
            .key_context("WorkspaceGroupPanel")
            .track_focus(&self.focus_handle)
            .size_full()
            .bg(colors.surface_background)
            // Enter → 편집 확정, Escape → 편집 취소
            .on_action(cx.listener(|this, _: &menu::Confirm, window, cx| {
                if this.editing.is_some() {
                    this.confirm_rename(window, cx);
                }
            }))
            .on_action(cx.listener(|this, _: &menu::Cancel, _window, cx| {
                if this.editing.is_some() {
                    this.cancel_rename(cx);
                }
            }))
            .child(
                // 상단 헤더: 제목 + 추가 버튼
                h_flex()
                    .w_full()
                    .px_2()
                    .py_1()
                    .gap_1()
                    .justify_between()
                    .border_b_1()
                    .border_color(colors.border)
                    .child(
                        div()
                            .text_sm()
                            .text_color(colors.text_muted)
                            .child(label_title.clone()),
                    )
                    .child(
                        IconButton::new("add-workspace-group", IconName::Plus)
                            .icon_size(ui::IconSize::Small)
                            .tooltip(Tooltip::text(label_add.clone()))
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.add_group(window, cx);
                            })),
                    ),
            )
            .child(
                // 그룹 목록
                v_flex()
                    .id("workspace-group-list")
                    .w_full()
                    .flex_1()
                    .overflow_y_scroll()
                    .py_1()
                    .children(groups.into_iter().map(|(index, name, has_notification)| {
                        let is_active = index == active_index;
                        let is_editing = editing_index == Some(index);
                        let can_delete = group_count > 1;
                        let name_for_menu = name.clone();
                        let name_for_rename = name.clone();
                        let name_shared: SharedString = name.clone().into();

                        // 드래그 데이터·리스너를 미리 생성
                        let drag_data = DraggedWorkspaceGroup {
                            index,
                            name: name_shared.clone(),
                        };
                        let drop_listener = cx.listener(
                            move |this, dragged: &DraggedWorkspaceGroup, window, cx| {
                                this.move_group(dragged.index, index, window, cx);
                            },
                        );

                        h_flex()
                            .id(("workspace-group-item", index))
                            .w_full()
                            .px_2()
                            .py(px(4.))
                            .gap_1()
                            .justify_between()
                            .rounded_md()
                            .cursor_pointer()
                            .when(is_active && !is_editing, |el| {
                                el.bg(colors.element_selected)
                            })
                            .hover(|el| {
                                if !is_active && !is_editing {
                                    el.bg(colors.element_hover)
                                } else {
                                    el
                                }
                            })
                            .when(!is_editing, |el| {
                                el.on_click(cx.listener(move |this, event: &gpui::ClickEvent, window, cx| {
                                    if event.click_count() == 2 {
                                        // 더블클릭 → 이름 변경
                                        this.start_rename(index, name_for_rename.clone(), window, cx);
                                    } else {
                                        this.switch_group(index, window, cx);
                                    }
                                }))
                            })
                            // 드래그 앤 드롭 — 편집 중이 아닌 항목만
                            .when(!is_editing, |el| {
                                el.on_drag(drag_data, |info, _, _, cx| {
                                    cx.new(|_| info.clone())
                                })
                                .drag_over::<DraggedWorkspaceGroup>(|style, _, _, _| {
                                    style.bg(gpui::rgba(0x3388ff20))
                                })
                                .on_drop(drop_listener)
                            })
                            // 우클릭 → 컨텍스트 메뉴
                            .on_mouse_down(
                                MouseButton::Right,
                                cx.listener(move |this, event: &gpui::MouseDownEvent, window, cx| {
                                    this.deploy_context_menu(
                                        event.position,
                                        index,
                                        name_for_menu.clone(),
                                        group_count,
                                        window,
                                        cx,
                                    );
                                }),
                            )
                            .child(
                                h_flex()
                                    .gap_1()
                                    .min_w_0()
                                    .flex_1()
                                    .overflow_x_hidden()
                                    .child(if is_editing {
                                        // 인라인 편집기 표시
                                        let editor = self.editing.as_ref().unwrap().1.clone();
                                        div()
                                            .w_full()
                                            .px_1()
                                            .py(px(1.))
                                            .rounded_sm()
                                            .border_1()
                                            .when(rename_error, |el| {
                                                el.border_color(gpui::red())
                                            })
                                            .when(!rename_error, |el| {
                                                el.border_color(colors.border_focused)
                                            })
                                            .bg(colors.editor_background)
                                            .text_sm()
                                            .text_color(colors.text)
                                            .child(editor)
                                            .into_any_element()
                                    } else {
                                        // 일반 이름 표시
                                        div()
                                            .text_sm()
                                            .when(is_active, |el| {
                                                el.text_color(colors.text)
                                            })
                                            .when(!is_active, |el| {
                                                el.text_color(colors.text_muted)
                                            })
                                            .child(name)
                                            .into_any_element()
                                    }),
                            )
                            // 비활성 그룹 알림 아이콘
                            .when(!is_active && has_notification && !is_editing, |el| {
                                el.child(
                                    ui::Icon::new(IconName::BellDot)
                                        .size(ui::IconSize::XSmall)
                                        .color(ui::Color::Accent),
                                )
                            })
                            .when(can_delete && !is_editing, |el| {
                                el.child(
                                    IconButton::new(
                                        ("remove-group", index),
                                        IconName::Close,
                                    )
                                    .icon_size(ui::IconSize::XSmall)
                                    .tooltip(Tooltip::text(label_delete_tooltip.clone()))
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        this.remove_group(index, window, cx);
                                    })),
                                )
                            })
                    })),
            )
            // 에러 메시지 표시
            .when(rename_error, |el| {
                el.child(
                    div()
                        .w_full()
                        .px_2()
                        .py_1()
                        .text_xs()
                        .text_color(gpui::red())
                        .child(label_rename_error.clone()),
                )
            })
            // 컨텍스트 메뉴 오버레이
            .children(self.context_menu.as_ref().map(|(menu, position, _)| {
                gpui::deferred(
                    gpui::anchored()
                        .position(*position)
                        .anchor(gpui::Corner::TopLeft)
                        .child(menu.clone()),
                )
                .with_priority(3)
            }))
    }
}
