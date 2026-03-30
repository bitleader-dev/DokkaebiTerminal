// 워크스페이스 그룹 패널 — 좌측 독에 표시되는 워크스페이스 그룹 목록 관리 UI

use gpui::{
    Action, App, Context, Entity, EventEmitter, FocusHandle, Focusable, InteractiveElement,
    IntoElement, ParentElement, Render, Styled, WeakEntity, Window, actions, px,
};
use ui::{
    Clickable, FluentBuilder, IconButton, IconName, Tooltip,
    prelude::*,
};

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

pub struct WorkspaceGroupPanel {
    workspace: WeakEntity<Workspace>,
    focus_handle: FocusHandle,
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
        cx.new(|cx| {
            Self {
                workspace: workspace_handle,
                focus_handle: cx.focus_handle(),
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
        Some("워크스페이스 그룹")
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
                let groups: Vec<(usize, String)> = ws
                    .workspace_groups()
                    .iter()
                    .enumerate()
                    .map(|(i, g)| (i, g.name.clone()))
                    .collect();
                let active = ws.active_group_index();
                let count = ws.workspace_group_count();
                (groups, active, count)
            } else {
                (Vec::new(), 0, 0)
            };

        let colors = cx.theme().colors();

        v_flex()
            .id("workspace-group-panel")
            .key_context("WorkspaceGroupPanel")
            .track_focus(&self.focus_handle)
            .size_full()
            .bg(colors.surface_background)
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
                            .child("워크스페이스"),
                    )
                    .child(
                        IconButton::new("add-workspace-group", IconName::Plus)
                            .icon_size(ui::IconSize::Small)
                            .tooltip(Tooltip::text("워크스페이스 추가"))
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
                    .children(groups.into_iter().map(|(index, name)| {
                        let is_active = index == active_index;
                        let can_delete = group_count > 1;

                        h_flex()
                            .id(("workspace-group-item", index))
                            .w_full()
                            .px_2()
                            .py(px(4.))
                            .gap_1()
                            .justify_between()
                            .rounded_md()
                            .cursor_pointer()
                            .when(is_active, |el| {
                                el.bg(colors.element_selected)
                            })
                            .hover(|el| {
                                if !is_active {
                                    el.bg(colors.element_hover)
                                } else {
                                    el
                                }
                            })
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.switch_group(index, window, cx);
                            }))
                            .child(
                                h_flex()
                                    .gap_1()
                                    .min_w_0()
                                    .flex_1()
                                    .overflow_x_hidden()
                                    .child(
                                        div()
                                            .text_sm()
                                            .when(is_active, |el| {
                                                el.text_color(colors.text)
                                            })
                                            .when(!is_active, |el| {
                                                el.text_color(colors.text_muted)
                                            })
                                            .child(name),
                                    ),
                            )
                            .when(can_delete, |el| {
                                el.child(
                                    IconButton::new(
                                        ("remove-group", index),
                                        IconName::Close,
                                    )
                                    .icon_size(ui::IconSize::XSmall)
                                    .tooltip(Tooltip::text("워크스페이스 삭제"))
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        this.remove_group(index, window, cx);
                                    })),
                                )
                            })
                    })),
            )
    }
}
