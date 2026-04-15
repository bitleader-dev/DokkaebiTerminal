use crate::{
    NewCenterTerminal, NewFile, Open, PathList, Pane, SerializedWorkspaceLocation, Workspace,
    WorkspaceId,
    item::{Item, ItemEvent},
    pane,
    persistence::WorkspaceDb,
};
use i18n::t;
use chrono::{DateTime, Utc};
use git::Clone as GitClone;
use gpui::WeakEntity;
use gpui::{
    Action, App, Context, Entity, EventEmitter, FocusHandle, Focusable, InteractiveElement,
    ParentElement, PathPromptOptions, Render, Styled, Subscription, Task, Window, actions, img,
};
use menu::{SelectNext, SelectPrevious};
use project::DirectoryLister;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ui::{ButtonLike, Divider, DividerColor, KeyBinding, prelude::*};
use util::ResultExt;
use zed_actions::{OpenOnboarding, OpenSettings};

#[derive(PartialEq, Clone, Debug, Deserialize, Serialize, JsonSchema, Action)]
#[action(namespace = welcome)]
#[serde(transparent)]
pub struct OpenRecentProject {
    pub index: usize,
}

actions!(
    dokkaebi,
    [
        /// Show the Zed welcome screen
        ShowWelcome
    ]
);

#[derive(IntoElement)]
struct SectionHeader {
    title: SharedString,
}

impl SectionHeader {
    fn new(title: impl Into<SharedString>) -> Self {
        Self {
            title: title.into(),
        }
    }
}

impl RenderOnce for SectionHeader {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        h_flex()
            .px_1()
            .mb_2()
            .gap_2()
            .child(
                Label::new(self.title.to_ascii_uppercase())
                    .buffer_font(cx)
                    .color(Color::Muted)
                    .size(LabelSize::XSmall),
            )
            .child(Divider::horizontal().color(DividerColor::BorderVariant))
    }
}

#[derive(IntoElement)]
struct SectionButton {
    label: SharedString,
    icon: IconName,
    action: Box<dyn Action>,
    tab_index: usize,
    focus_handle: FocusHandle,
    disabled: bool,
}

impl SectionButton {
    fn new(
        label: impl Into<SharedString>,
        icon: IconName,
        action: &dyn Action,
        tab_index: usize,
        focus_handle: FocusHandle,
    ) -> Self {
        Self {
            label: label.into(),
            icon,
            action: action.boxed_clone(),
            tab_index,
            focus_handle,
            disabled: false,
        }
    }

    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl RenderOnce for SectionButton {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let id = format!("onb-button-{}-{}", self.label, self.tab_index);
        let action_ref: &dyn Action = &*self.action;

        ButtonLike::new(id)
            .tab_index(self.tab_index as isize)
            .full_width()
            .size(ButtonSize::Medium)
            .disabled(self.disabled)
            .child(
                h_flex()
                    .w_full()
                    .justify_between()
                    .child(
                        h_flex()
                            .gap_2()
                            .child(
                                Icon::new(self.icon)
                                    .color(Color::Muted)
                                    .size(IconSize::Small),
                            )
                            .child(Label::new(self.label)),
                    )
                    .child(
                        KeyBinding::for_action_in(action_ref, &self.focus_handle, cx)
                            .size(rems_from_px(12.)),
                    ),
            )
            .on_click(move |_, window, cx| {
                self.focus_handle.dispatch_action(&*self.action, window, cx)
            })
    }
}

/// 환영 탭 도움말 섹션의 정보 행. 클릭 불가하며 좌측 아이콘·설명, 우측 안내 텍스트를 보여준다.
#[derive(IntoElement)]
struct InfoRow {
    label: SharedString,
    value: SharedString,
    icon: IconName,
}

impl InfoRow {
    fn new(
        label: impl Into<SharedString>,
        value: impl Into<SharedString>,
        icon: IconName,
    ) -> Self {
        Self {
            label: label.into(),
            value: value.into(),
            icon,
        }
    }
}

impl RenderOnce for InfoRow {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        // 우측 값은 Label 대신 div+text 스타일로 직접 렌더해야 flex 자식의 min-content 제약을
        // 받지 않고 자유롭게 줄바꿈된다.
        let value_color = Color::Muted.color(cx);
        let value_font = theme::theme_settings(cx).buffer_font(cx).clone();
        h_flex()
            .w_full()
            .min_h(rems_from_px(28.))
            .px_2()
            .gap_4()
            .items_start()
            .child(
                h_flex()
                    .flex_shrink_0()
                    .gap_2()
                    .child(
                        Icon::new(self.icon)
                            .color(Color::Muted)
                            .size(IconSize::Small),
                    )
                    .child(Label::new(self.label)),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .text_right()
                    .text_ui_sm(cx)
                    .text_color(value_color)
                    .font(value_font)
                    .child(self.value),
            )
    }
}

enum SectionVisibility {
    Always,
}

impl SectionVisibility {
    fn is_visible(&self, _cx: &App) -> bool {
        match self {
            SectionVisibility::Always => true,
        }
    }
}

struct SectionEntry {
    icon: IconName,
    title: &'static str,
    action: &'static dyn Action,
    visibility_guard: SectionVisibility,
}

impl SectionEntry {
    fn render(
        &self,
        button_index: usize,
        focus: &FocusHandle,
        disabled: bool,
        cx: &App,
    ) -> Option<impl IntoElement> {
        self.visibility_guard.is_visible(cx).then(|| {
            SectionButton::new(
                t(self.title, cx),
                self.icon,
                self.action,
                button_index,
                focus.clone(),
            )
            .disabled(disabled)
        })
    }
}

const CONTENT: (Section<4>, Section<1>) = (
    Section {
        title: "welcome.section.get_started",
        entries: [
            SectionEntry {
                icon: IconName::Terminal,
                title: "welcome.action.new_terminal",
                action: &NewCenterTerminal { local: false },
                visibility_guard: SectionVisibility::Always,
            },
            SectionEntry {
                icon: IconName::Plus,
                title: "welcome.action.new_file",
                action: &NewFile,
                visibility_guard: SectionVisibility::Always,
            },
            SectionEntry {
                icon: IconName::FolderOpen,
                title: "welcome.action.open_project",
                action: &Open::DEFAULT,
                visibility_guard: SectionVisibility::Always,
            },
            SectionEntry {
                icon: IconName::CloudDownload,
                title: "welcome.action.clone_repository",
                action: &GitClone,
                visibility_guard: SectionVisibility::Always,
            },
        ],
    },
    Section {
        title: "welcome.section.configure",
        entries: [
            SectionEntry {
                icon: IconName::Settings,
                title: "welcome.action.open_settings",
                action: &OpenSettings,
                visibility_guard: SectionVisibility::Always,
            },
        ],
    },
);

struct Section<const COLS: usize> {
    title: &'static str,
    entries: [SectionEntry; COLS],
}

impl<const COLS: usize> Section<COLS> {
    fn render(
        self,
        index_offset: usize,
        focus: &FocusHandle,
        disabled: bool,
        cx: &App,
    ) -> impl IntoElement {
        v_flex()
            .min_w_full()
            .child(SectionHeader::new(t(self.title, cx)))
            .children(self.entries.iter().enumerate().filter_map(|(index, entry)| {
                entry.render(index_offset + index, focus, disabled, cx)
            }))
    }
}

pub struct WelcomePage {
    workspace: WeakEntity<Workspace>,
    focus_handle: FocusHandle,
    fallback_to_recent_projects: bool,
    recent_workspaces: Option<
        Vec<(
            WorkspaceId,
            SerializedWorkspaceLocation,
            PathList,
            DateTime<Utc>,
        )>,
    >,
    /// 새 터미널 생성 진행 여부 (중복 클릭 방지)
    creating_terminal: bool,
    /// 새 파일 생성 진행 여부 (중복 클릭 방지)
    creating_file: bool,
    /// 활성 pane의 AddItem 이벤트 구독 핸들
    pane_subscription: Option<Subscription>,
}

impl WelcomePage {
    pub fn new(
        workspace: WeakEntity<Workspace>,
        fallback_to_recent_projects: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let focus_handle = cx.focus_handle();
        cx.on_focus(&focus_handle, window, |_, _, cx| cx.notify())
            .detach();

        if fallback_to_recent_projects {
            let fs = workspace
                .upgrade()
                .map(|ws| ws.read(cx).app_state().fs.clone());
            let db = WorkspaceDb::global(cx);
            cx.spawn_in(window, async move |this: WeakEntity<Self>, cx| {
                let Some(fs) = fs else { return };
                let workspaces = db
                    .recent_workspaces_on_disk(fs.as_ref())
                    .await
                    .log_err()
                    .unwrap_or_default();

                this.update(cx, |this, cx| {
                    this.recent_workspaces = Some(workspaces);
                    cx.notify();
                })
                .ok();
            })
            .detach();
        }

        WelcomePage {
            workspace,
            focus_handle,
            fallback_to_recent_projects,
            recent_workspaces: None,
            creating_terminal: false,
            creating_file: false,
            pane_subscription: None,
        }
    }

    fn select_next(&mut self, _: &SelectNext, window: &mut Window, cx: &mut Context<Self>) {
        window.focus_next(cx);
        cx.notify();
    }

    fn select_previous(&mut self, _: &SelectPrevious, window: &mut Window, cx: &mut Context<Self>) {
        window.focus_prev(cx);
        cx.notify();
    }

    /// 활성 pane에 새 항목이 추가되면 환영 탭을 닫고 비지 상태를 해제
    fn watch_active_pane_for_close(&mut self, cx: &mut Context<Self>) {
        let Some(workspace) = self.workspace.upgrade() else {
            return;
        };
        let active_pane = workspace.read(cx).active_pane().clone();
        let subscription = cx.subscribe(
            &active_pane,
            |this, _pane: Entity<Pane>, event: &pane::Event, cx| {
                if let pane::Event::AddItem { .. } = event {
                    this.creating_terminal = false;
                    this.creating_file = false;
                    this.pane_subscription = None;
                    cx.emit(ItemEvent::CloseItem);
                }
            },
        );
        self.pane_subscription = Some(subscription);
    }

    /// 새 파일 액션 처리: 중복 클릭 방지 + 항목 추가 후 환영 탭 닫기
    fn on_new_file(&mut self, _: &NewFile, _window: &mut Window, cx: &mut Context<Self>) {
        if self.creating_terminal || self.creating_file {
            // 비지 상태이므로 여기서 액션을 차단 (propagate 호출하지 않음)
            return;
        }
        self.creating_file = true;
        self.watch_active_pane_for_close(cx);
        cx.notify();
        // 워크스페이스의 NewFile 핸들러로 액션이 전달되도록 propagate
        cx.propagate();
    }

    /// 새 터미널 액션 처리: 중복 클릭 방지 + 항목 추가 후 환영 탭 닫기
    fn on_new_center_terminal(
        &mut self,
        _: &NewCenterTerminal,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.creating_terminal || self.creating_file {
            // 비지 상태이므로 여기서 액션을 차단 (propagate 호출하지 않음)
            return;
        }
        self.creating_terminal = true;
        self.watch_active_pane_for_close(cx);
        cx.notify();
        // 워크스페이스의 NewCenterTerminal 핸들러로 액션이 전달되도록 propagate
        cx.propagate();
    }

    /// 프로젝트 열기 액션 인터셉트: 프로젝트 열기 + 센터 터미널 추가
    fn on_open_project(&mut self, _: &Open, window: &mut Window, cx: &mut Context<Self>) {
        cx.stop_propagation();
        cx.emit(ItemEvent::CloseItem);

        let workspace = self.workspace.clone();
        let Some(ws) = workspace.upgrade() else {
            return;
        };
        let app_state = ws.read(cx).app_state().clone();

        let paths_task = ws.update(cx, |ws, cx| {
            let project = ws.project().clone();
            let fs = app_state.fs.clone();
            ws.prompt_for_open_path(
                PathPromptOptions {
                    files: true,
                    directories: true,
                    multiple: true,
                    prompt: None,
                },
                DirectoryLister::Local(project, fs),
                window,
                cx,
            )
        });

        cx.spawn_in(window, async move |_this, cx| {
            let Some(paths) = paths_task.await.log_err().flatten() else {
                return;
            };

            // 프로젝트 열기
            let open_result = workspace.update_in(cx, |ws, window, cx| {
                ws.open_workspace_for_paths(false, paths, window, cx)
            });

            if let Ok(task) = open_result {
                if let Some(_new_ws) = task.await.log_err() {
                    // 새 워크스페이스에 센터 터미널 추가
                    cx.update(|window, cx| {
                        window.dispatch_action(
                            NewCenterTerminal { local: false }.boxed_clone(),
                            cx,
                        );
                    })
                    .ok();
                }
            }
        })
        .detach();
    }

    fn open_recent_project(
        &mut self,
        action: &OpenRecentProject,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(recent_workspaces) = &self.recent_workspaces {
            if let Some((_workspace_id, location, paths, _timestamp)) =
                recent_workspaces.get(action.index)
            {
                let is_local = matches!(location, SerializedWorkspaceLocation::Local);

                if is_local {
                    let paths = paths.clone();
                    let paths = paths.paths().to_vec();
                    self.workspace
                        .update(cx, |workspace, cx| {
                            workspace
                                .open_workspace_for_paths(true, paths, window, cx)
                                .detach_and_log_err(cx);
                        })
                        .log_err();
                } else {
                    use zed_actions::OpenRecent;
                    window.dispatch_action(OpenRecent::default().boxed_clone(), cx);
                }
            }
        }
    }

    /// 도움말 섹션 렌더링. 클릭 불가한 정보 행 4개를 고정 순서로 표시한다.
    fn render_help_section(&self, cx: &App) -> impl IntoElement {
        v_flex()
            .min_w_full()
            .child(SectionHeader::new(t("welcome.section.help", cx)))
            .child(InfoRow::new(
                t("welcome.help.prompt_palette.label", cx),
                t("welcome.help.prompt_palette.value", cx),
                IconName::Sparkle,
            ))
            .child(InfoRow::new(
                t("welcome.help.terminal_history.label", cx),
                t("welcome.help.terminal_history.value", cx),
                IconName::HistoryRerun,
            ))
            .child(InfoRow::new(
                t("welcome.help.claude_code_sound.label", cx),
                t("welcome.help.claude_code_sound.value", cx),
                IconName::Bell,
            ))
            .child(InfoRow::new(
                t("welcome.help.background_image.label", cx),
                t("welcome.help.background_image.value", cx),
                IconName::Image,
            ))
    }

    fn render_recent_project_section(
        &self,
        recent_projects: Vec<impl IntoElement>,
        cx: &App,
    ) -> impl IntoElement {
        v_flex()
            .w_full()
            .child(SectionHeader::new(t("welcome.recent_projects", cx)))
            .children(recent_projects)
    }

    fn render_recent_project(
        &self,
        project_index: usize,
        tab_index: usize,
        location: &SerializedWorkspaceLocation,
        paths: &PathList,
        cx: &App,
    ) -> impl IntoElement {
        let (icon, title) = match location {
            SerializedWorkspaceLocation::Local => {
                let path = paths.paths().first().map(|p| p.as_path());
                let name = path
                    .and_then(|p| p.file_name())
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "Untitled".to_string());
                (IconName::Folder, name)
            }
            SerializedWorkspaceLocation::Remote(_) => {
                (IconName::Server, t("welcome.remote_project", cx).to_string())
            }
        };

        SectionButton::new(
            title,
            icon,
            &OpenRecentProject {
                index: project_index,
            },
            tab_index,
            self.focus_handle.clone(),
        )
    }
}

impl Render for WelcomePage {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (first_section, second_section) = CONTENT;
        let first_section_entries = first_section.entries.len();
        let last_index = first_section_entries + second_section.entries.len();

        let recent_projects = self
            .recent_workspaces
            .as_ref()
            .into_iter()
            .flatten()
            .take(5)
            .enumerate()
            .map(|(index, (_, loc, paths, _))| {
                self.render_recent_project(index, first_section_entries + index, loc, paths, cx)
            })
            .collect::<Vec<_>>();

        // 새 파일/터미널 생성 중일 때는 시작하기 섹션 버튼을 모두 비활성화
        let first_section_disabled = self.creating_terminal || self.creating_file;

        let second_section = if self.fallback_to_recent_projects && !recent_projects.is_empty() {
            self.render_recent_project_section(recent_projects, cx)
                .into_any_element()
        } else {
            second_section
                .render(first_section_entries, &self.focus_handle, false, cx)
                .into_any_element()
        };

        let welcome_key = if self.fallback_to_recent_projects {
            "welcome.headline.returning"
        } else {
            "welcome.headline.first_time"
        };
        let welcome_label = t(welcome_key, cx);

        h_flex()
            .key_context("Welcome")
            .track_focus(&self.focus_handle(cx))
            .on_action(cx.listener(Self::select_previous))
            .on_action(cx.listener(Self::select_next))
            .on_action(cx.listener(Self::open_recent_project))
            .on_action(cx.listener(Self::on_new_file))
            .on_action(cx.listener(Self::on_new_center_terminal))
            .on_action(cx.listener(Self::on_open_project))
            .size_full()
            .justify_center()
            .overflow_hidden()
            .bg(cx.theme().colors().editor_background)
            .child(
                h_flex()
                    .relative()
                    .size_full()
                    .px_12()
                    .max_w(px(1100.))
                    .child(
                        v_flex()
                            .flex_1()
                            .justify_center()
                            .max_w_128()
                            .mx_auto()
                            .gap_6()
                            .overflow_x_hidden()
                            .child(
                                h_flex()
                                    .w_full()
                                    .justify_center()
                                    .mb_4()
                                    .gap_4()
                                    .child(
                                        img("icons/icon.png")
                                            .w(rems_from_px(70.))
                                            .h(rems_from_px(70.))
                                            .flex_none(),
                                    )
                                    .child(
                                        v_flex().child(Headline::new(welcome_label)).child(
                                            Label::new(t("welcome.tagline", cx))
                                                .size(LabelSize::Small)
                                                .color(Color::Muted)
                                                .italic(),
                                        ),
                                    ),
                            )
                            .child(first_section.render(
                                Default::default(),
                                &self.focus_handle,
                                first_section_disabled,
                                cx,
                            ))
                            .child(second_section)
                            .when(!self.fallback_to_recent_projects, |this| {
                                this.child(self.render_help_section(cx)).child(
                                    v_flex().gap_1().child(Divider::horizontal()).child(
                                        Button::new("welcome-exit", t("welcome.return_to_onboarding", cx))
                                            .tab_index(last_index as isize)
                                            .full_width()
                                            .label_size(LabelSize::XSmall)
                                            .on_click(|_, window, cx| {
                                                window.dispatch_action(
                                                    OpenOnboarding.boxed_clone(),
                                                    cx,
                                                );
                                            }),
                                    ),
                                )
                            }),
                    ),
            )
    }
}

impl EventEmitter<ItemEvent> for WelcomePage {}

impl Focusable for WelcomePage {
    fn focus_handle(&self, _: &App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Item for WelcomePage {
    type Event = ItemEvent;

    fn tab_content_text(&self, _detail: usize, cx: &App) -> SharedString {
        t("welcome.tab_title", cx)
    }

    fn telemetry_event_text(&self) -> Option<&'static str> {
        Some("New Welcome Page Opened")
    }

    fn show_toolbar(&self) -> bool {
        false
    }

    fn to_item_events(event: &Self::Event, f: &mut dyn FnMut(crate::item::ItemEvent)) {
        f(*event)
    }
}

impl crate::SerializableItem for WelcomePage {
    fn serialized_item_kind() -> &'static str {
        "WelcomePage"
    }

    fn cleanup(
        workspace_id: crate::WorkspaceId,
        alive_items: Vec<crate::ItemId>,
        _window: &mut Window,
        cx: &mut App,
    ) -> Task<gpui::Result<()>> {
        crate::delete_unloaded_items(
            alive_items,
            workspace_id,
            "welcome_pages",
            &persistence::WelcomePagesDb::global(cx),
            cx,
        )
    }

    fn deserialize(
        _project: Entity<project::Project>,
        workspace: gpui::WeakEntity<Workspace>,
        workspace_id: crate::WorkspaceId,
        item_id: crate::ItemId,
        window: &mut Window,
        cx: &mut App,
    ) -> Task<gpui::Result<Entity<Self>>> {
        if persistence::WelcomePagesDb::global(cx)
            .get_welcome_page(item_id, workspace_id)
            .ok()
            .is_some_and(|is_open| is_open)
        {
            Task::ready(Ok(
                cx.new(|cx| WelcomePage::new(workspace, false, window, cx))
            ))
        } else {
            Task::ready(Err(anyhow::anyhow!("No welcome page to deserialize")))
        }
    }

    fn serialize(
        &mut self,
        workspace: &mut Workspace,
        item_id: crate::ItemId,
        _closing: bool,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<Task<gpui::Result<()>>> {
        let workspace_id = workspace.database_id()?;
        let db = persistence::WelcomePagesDb::global(cx);
        Some(cx.background_spawn(
            async move { db.save_welcome_page(item_id, workspace_id, true).await },
        ))
    }

    fn should_serialize(&self, event: &Self::Event) -> bool {
        event == &ItemEvent::UpdateTab
    }
}

mod persistence {
    use crate::WorkspaceDb;
    use db::{
        query,
        sqlez::{domain::Domain, thread_safe_connection::ThreadSafeConnection},
        sqlez_macros::sql,
    };

    pub struct WelcomePagesDb(ThreadSafeConnection);

    impl Domain for WelcomePagesDb {
        const NAME: &str = stringify!(WelcomePagesDb);

        const MIGRATIONS: &[&str] = (&[sql!(
                    CREATE TABLE welcome_pages (
                        workspace_id INTEGER,
                        item_id INTEGER UNIQUE,
                        is_open INTEGER DEFAULT FALSE,

                        PRIMARY KEY(workspace_id, item_id),
                        FOREIGN KEY(workspace_id) REFERENCES workspaces(workspace_id)
                        ON DELETE CASCADE
                    ) STRICT;
        )]);
    }

    db::static_connection!(WelcomePagesDb, [WorkspaceDb]);

    impl WelcomePagesDb {
        query! {
            pub async fn save_welcome_page(
                item_id: crate::ItemId,
                workspace_id: crate::WorkspaceId,
                is_open: bool
            ) -> Result<()> {
                INSERT OR REPLACE INTO welcome_pages(item_id, workspace_id, is_open)
                VALUES (?, ?, ?)
            }
        }

        query! {
            pub fn get_welcome_page(
                item_id: crate::ItemId,
                workspace_id: crate::WorkspaceId
            ) -> Result<bool> {
                SELECT is_open
                FROM welcome_pages
                WHERE item_id = ? AND workspace_id = ?
            }
        }
    }
}
