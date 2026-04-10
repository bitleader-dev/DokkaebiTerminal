mod application_menu;

mod onboarding_banner;
mod title_bar_settings;
mod update_version;

#[cfg(feature = "stories")]
mod stories;

use crate::application_menu::{ApplicationMenu, show_menus};
pub use platform_title_bar::{
    self, DraggedWindowTab, MergeAllWindows, MoveTabToNewWindow, PlatformTitleBar,
    ShowNextWindowTab, ShowPreviousWindowTab,
};
use project::linked_worktree_short_name;


use gpui::{
    Action, AnyElement, App, Context, Entity, Focusable, InteractiveElement,
    IntoElement, MouseButton, ParentElement, Render, Styled,
    Subscription, WeakEntity, Window, actions, div,
};
use onboarding_banner::OnboardingBanner;
use project::{Project, git_store::GitStoreEvent, trusted_worktrees::TrustedWorktrees};
use settings::Settings;
use settings::WorktreeId;
use std::collections::HashSet;
use std::sync::Arc;
use theme::ActiveTheme;
use title_bar_settings::TitleBarSettings;
use ui::{
    ButtonLike, ContextMenu, IconWithIndicator, Indicator, PopoverMenu, PopoverMenuHandle,
    TintColor, Tooltip, prelude::*, utils::platform_title_bar_height,
};
use update_version::UpdateVersion;
use util::ResultExt;
use workspace::{
    MultiWorkspace, ToggleWorktreeSecurity, Workspace, WorkspaceId,
    workspace_group_panel::ToggleWorkspaceGroupPanel,
};
use zed_actions::OpenRemote;

pub use onboarding_banner::restore_banner;

#[cfg(feature = "stories")]
pub use stories::*;

const MAX_PROJECT_NAME_LENGTH: usize = 40;
const MAX_BRANCH_NAME_LENGTH: usize = 40;
const MAX_SHORT_SHA_LENGTH: usize = 8;

actions!(
    title_bar,
    [
        /// A debug action to simulate an update being available to test the update banner UI.
        SimulateUpdateAvailable
    ]
);

pub fn init(cx: &mut App) {
    platform_title_bar::PlatformTitleBar::init(cx);

    cx.observe_new(|workspace: &mut Workspace, window, cx| {
        let Some(window) = window else {
            return;
        };
        let multi_workspace = workspace.multi_workspace().cloned();
        let item = cx.new(|cx| TitleBar::new("title-bar", workspace, multi_workspace, window, cx));
        workspace.set_titlebar_item(item.into(), window, cx);

        workspace.register_action(|workspace, _: &SimulateUpdateAvailable, _window, cx| {
            if let Some(titlebar) = workspace
                .titlebar_item()
                .and_then(|item| item.downcast::<TitleBar>().ok())
            {
                titlebar.update(cx, |titlebar, cx| {
                    titlebar.toggle_update_simulation(cx);
                });
            }
        });
    })
    .detach();
}

pub struct TitleBar {
    platform_titlebar: Entity<PlatformTitleBar>,
    project: Entity<Project>,
    workspace: WeakEntity<Workspace>,
    multi_workspace: Option<WeakEntity<MultiWorkspace>>,
    /// 타이틀바 오른쪽 설정 버튼 메뉴 핸들
    settings_menu_handle: PopoverMenuHandle<ContextMenu>,
    _subscriptions: Vec<Subscription>,
    banner: Entity<OnboardingBanner>,
    update_version: Entity<UpdateVersion>,

}

impl Render for TitleBar {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.multi_workspace.is_none() {
            if let Some(mw) = self
                .workspace
                .upgrade()
                .and_then(|ws| ws.read(cx).multi_workspace().cloned())
            {
                self.multi_workspace = Some(mw.clone());
                self.platform_titlebar.update(cx, |titlebar, _cx| {
                    titlebar.set_multi_workspace(mw);
                });
            }
        }

        let title_bar_settings = *TitleBarSettings::get_global(cx);
        let button_layout = title_bar_settings.button_layout;

        let show_menus = show_menus(cx);

        let mut children = Vec::new();

        let mut project_name = None;
        let mut repository = None;
        let mut linked_worktree_name = None;
        if let Some(worktree) = self.effective_active_worktree(cx) {
            repository = self.get_repository_for_worktree(&worktree, cx);
            let worktree = worktree.read(cx);
            project_name = worktree
                .root_name()
                .file_name()
                .map(|name| SharedString::from(name.to_string()));
            linked_worktree_name = repository.as_ref().and_then(|repo| {
                let repo = repo.read(cx);
                linked_worktree_short_name(
                    repo.original_repo_abs_path.as_ref(),
                    repo.work_directory_abs_path.as_ref(),
                )
                .filter(|name| Some(name) != project_name.as_ref())
            });
        }

        // 왼쪽: 패널 토글 버튼
        {
            let workspace = self.workspace.clone();
            children.push(
                h_flex()
                    .h_full()
                    .gap_0p5()
                    .child(
                        IconButton::new("toggle-workspace-group-panel", ui::IconName::ThreadsSidebarLeftClosed)
                            .style(ButtonStyle::Subtle)
                            .icon_size(IconSize::Small)
                            .tooltip(Tooltip::for_action_title(
                                "워크스페이스 그룹 패널",
                                &ToggleWorkspaceGroupPanel,
                            ))
                            .on_click(move |_, window, cx| {
                                if let Some(ws) = workspace.upgrade() {
                                    ws.update(cx, |ws, cx| {
                                        let is_open = ws.left_dock().read(cx).is_open()
                                            && ws.left_dock().read(cx)
                                                .active_panel()
                                                .map_or(false, |p| {
                                                    p.persistent_name() == "WorkspaceGroupPanel"
                                                });
                                        if is_open {
                                            ws.close_panel::<workspace::WorkspaceGroupPanel>(window, cx);
                                        } else {
                                            ws.open_panel::<workspace::WorkspaceGroupPanel>(window, cx);
                                        }
                                    });
                                }
                            }),
                    )
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .into_any_element(),
            );
        }

        // 가운데: 워크스페이스 그룹 이름 (flex-1로 남은 공간 차지 후 가운데 정렬)
        {
            let group_name = self
                .workspace
                .upgrade()
                .and_then(|ws| {
                    let ws = ws.read(cx);
                    let groups = ws.workspace_groups();
                    let active = ws.active_group_index();
                    groups.get(active).map(|g| g.name.clone())
                })
                .unwrap_or_default();

            children.push(
                h_flex()
                    .flex_1()
                    .h_full()
                    .justify_center()
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().colors().text_muted)
                            .child(group_name),
                    )
                    .into_any_element(),
            );
        }

        // 오른쪽: 설정 버튼
        children.push(
            h_flex()
                .pr_1()
                .gap_1()
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .child(self.update_version.clone())
                .child(self.render_settings_button(cx))
                .into_any_element(),
        );

        if show_menus {
            self.platform_titlebar.update(cx, |this, _| {
                this.set_button_layout(button_layout);
                this.set_children(None::<gpui::AnyElement>);
            });

            let height = platform_title_bar_height(window);
            let title_bar_color = self.platform_titlebar.update(cx, |platform_titlebar, cx| {
                platform_titlebar.title_bar_color(window, cx)
            });

            v_flex()
                .w_full()
                .child(self.platform_titlebar.clone().into_any_element())
                .child(
                    h_flex()
                        .bg(title_bar_color)
                        .h(height)
                        .pl_2()
                        .justify_between()
                        .w_full()
                        .children(children),
                )
                .into_any_element()
        } else {
            self.platform_titlebar.update(cx, |this, _| {
                this.set_button_layout(button_layout);
                this.set_children(children);
            });
            self.platform_titlebar.clone().into_any_element()
        }
    }
}

impl TitleBar {
    pub fn new(
        id: impl Into<ElementId>,
        workspace: &Workspace,
        multi_workspace: Option<WeakEntity<MultiWorkspace>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let project = workspace.project().clone();
        let git_store = project.read(cx).git_store().clone();
        let mut subscriptions = Vec::new();
        subscriptions.push(
            cx.observe(&workspace.weak_handle().upgrade().unwrap(), |_, _, cx| {
                cx.notify()
            }),
        );
        subscriptions.push(
            cx.subscribe(&project, |this, _, event: &project::Event, cx| {
                if let project::Event::BufferEdited = event {
                    // Clear override when user types in any editor,
                    // so the title bar reflects the project they're actually working in
                    this.clear_active_worktree_override(cx);
                    cx.notify();
                }
            }),
        );

        subscriptions.push(cx.observe_window_activation(window, Self::window_activation_changed));
        subscriptions.push(
            cx.subscribe(&git_store, move |this, _, event, cx| match event {
                GitStoreEvent::ActiveRepositoryChanged(_) => {
                    // Clear override when focus-derived active repo changes
                    // (meaning the user focused a file from a different project)
                    this.clear_active_worktree_override(cx);
                    cx.notify();
                }
                GitStoreEvent::RepositoryUpdated(_, _, true) => {
                    cx.notify();
                }
                _ => {}
            }),
        );
        subscriptions.push(cx.observe_button_layout_changed(window, |_, _, cx| cx.notify()));
        if let Some(trusted_worktrees) = TrustedWorktrees::try_get_global(cx) {
            subscriptions.push(cx.subscribe(&trusted_worktrees, |_, _, _, cx| {
                cx.notify();
            }));
        }

        let banner = cx.new(|cx| {
            OnboardingBanner::new(
                "ACP Claude Code Onboarding",
                IconName::AiClaude,
                "Claude Agent",
                Some("Introducing:".into()),
                zed_actions::agent::OpenClaudeAgentOnboardingModal.boxed_clone(),
                cx,
            )
            // When updating this to a non-AI feature release, remove this line.
            .visible_when(|cx| !project::DisableAiSettings::get_global(cx).disable_ai)
        });

        let update_version = cx.new(|cx| UpdateVersion::new(cx));
        let platform_titlebar = cx.new(|cx| {
            let mut titlebar = PlatformTitleBar::new(id, cx);
            if let Some(mw) = multi_workspace.clone() {
                titlebar = titlebar.with_multi_workspace(mw);
            }
            titlebar
        });

        let mut this = Self {
            platform_titlebar,
            workspace: workspace.weak_handle(),
            multi_workspace,
            project,
            settings_menu_handle: PopoverMenuHandle::default(),
            _subscriptions: subscriptions,
            banner,
            update_version,

        };


        this
    }

    /// 타이틀바 왼쪽에 표시되는 워크스페이스 그룹 전환 버튼
    fn render_workspace_group_button(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let workspace = self.workspace.clone();

        // 현재 워크스페이스에서 그룹 정보 읽기
        let (group_name, group_count) = self
            .workspace
            .upgrade()
            .map(|ws| {
                let ws = ws.read(cx);
                let groups = ws.workspace_groups();
                let active = ws.active_group_index();
                let name = groups
                    .get(active)
                    .map(|g| g.name.clone())
                    .unwrap_or_default();
                (name, groups.len())
            })
            .unwrap_or_default();

        let display_name: SharedString = group_name.into();

        div()
            .id("workspace-group-menu-item")
            .occlude()
            .child(
                PopoverMenu::new("workspace-group-menu-popover")
                    .menu(move |window, cx| {
                        let workspace = workspace.clone();
                        let ws = workspace.upgrade()?;
                        let ws_read = ws.read(cx);
                        let groups: Vec<(usize, String)> = ws_read
                            .workspace_groups()
                            .iter()
                            .enumerate()
                            .map(|(i, g)| (i, g.name.clone()))
                            .collect();
                        let active_index = ws_read.active_group_index();
                        let count = groups.len();

                        Some(ContextMenu::build(window, cx, move |mut menu, _window, _cx| {
                            for (index, name) in groups {
                                let is_active = index == active_index;
                                let ws_handle = workspace.clone();
                                menu = menu.toggleable_entry(
                                    SharedString::from(name),
                                    is_active,
                                    ui::IconPosition::Start,
                                    None,
                                    move |window, cx| {
                                        if let Some(ws) = ws_handle.upgrade() {
                                            ws.update(cx, |ws, cx| {
                                                ws.switch_workspace_group(index, window, cx);
                                            });
                                        }
                                    },
                                );
                            }

                            menu = menu.separator();

                            // 그룹 추가 버튼
                            let ws_add = workspace.clone();
                            menu = menu.entry(
                                "그룹 추가",
                                None,
                                move |window, cx| {
                                    if let Some(ws) = ws_add.upgrade() {
                                        ws.update(cx, |ws, cx| {
                                            ws.add_workspace_group(window, cx);
                                        });
                                    }
                                },
                            );

                            // 그룹이 2개 이상일 때만 삭제 버튼 표시
                            if count > 1 {
                                let ws_remove = workspace.clone();
                                let remove_index = active_index;
                                menu = menu.entry(
                                    "현재 그룹 삭제",
                                    None,
                                    move |window, cx| {
                                        if let Some(ws) = ws_remove.upgrade() {
                                            ws.update(cx, |ws, cx| {
                                                ws.remove_workspace_group(remove_index, window, cx);
                                            });
                                        }
                                    },
                                );
                            }

                            menu
                        }))
                    })
                    .trigger(
                        Button::new("workspace-group-trigger", display_name)
                            .style(ButtonStyle::Subtle)
                            .label_size(LabelSize::Small)
                            .when(group_count > 1, |btn| {
                                btn.end_icon(
                                    Icon::new(IconName::ChevronDown)
                                        .size(IconSize::XSmall)
                                        .color(Color::Muted),
                                )
                            })
                            .tooltip(Tooltip::text("워크스페이스 그룹")),
                    ),
            )
    }

    /// 타이틀바 오른쪽에 표시되는 설정 버튼 (Zed 메뉴 팝오버)
    fn render_settings_button(&self, _cx: &mut Context<Self>) -> impl IntoElement {
        let handle = self.settings_menu_handle.clone();

        div()
            .id("settings-menu-item")
            .occlude()
            .child(
                PopoverMenu::new("settings-menu-popover")
                    .menu(move |window, cx| {
                        let menus = cx.get_menus().unwrap_or_default();
                        let Some(zed_menu) = menus.into_iter().next() else {
                            return None;
                        };
                        let sanitized_items =
                            ApplicationMenu::sanitize_menu_items(zed_menu.items);
                        Some(ContextMenu::build(window, cx, |menu, window, cx| {
                            let menu = menu.when_some(
                                window.focused(cx),
                                |menu, focused| menu.context(focused),
                            );
                            sanitized_items
                                .into_iter()
                                .fold(menu, |menu, item| match item {
                                    gpui::OwnedMenuItem::Separator => menu.separator(),
                                    gpui::OwnedMenuItem::Action {
                                        name,
                                        action,
                                        checked,
                                        disabled,
                                        ..
                                    } => menu.action_checked_with_disabled(
                                        name, action, checked, disabled,
                                    ),
                                    gpui::OwnedMenuItem::Submenu(submenu) => submenu
                                        .items
                                        .into_iter()
                                        .fold(menu, |menu, item| match item {
                                            gpui::OwnedMenuItem::Separator => menu.separator(),
                                            gpui::OwnedMenuItem::Action {
                                                name,
                                                action,
                                                checked,
                                                disabled,
                                                ..
                                            } => menu.action_checked_with_disabled(
                                                name, action, checked, disabled,
                                            ),
                                            _ => menu,
                                        }),
                                    _ => menu,
                                })
                        }))
                    })
                    .trigger(
                        IconButton::new("settings-menu-trigger", ui::IconName::Settings)
                            .style(ButtonStyle::Subtle)
                            .icon_size(IconSize::Small)
                            .tooltip(Tooltip::text("설정")),
                    )
                    .with_handle(handle),
            )
    }

    fn worktree_count(&self, cx: &App) -> usize {
        self.project.read(cx).visible_worktrees(cx).count()
    }

    fn toggle_update_simulation(&mut self, cx: &mut Context<Self>) {
        self.update_version
            .update(cx, |banner, cx| banner.update_simulation(cx));
        cx.notify();
    }

    /// Returns the worktree to display in the title bar.
    /// - If there's an override set on the workspace, use that (if still valid)
    /// - Otherwise, derive from the active repository
    /// - Fall back to the first visible worktree
    pub fn effective_active_worktree(&self, cx: &App) -> Option<Entity<project::Worktree>> {
        let project = self.project.read(cx);

        if let Some(workspace) = self.workspace.upgrade() {
            if let Some(override_id) = workspace.read(cx).active_worktree_override() {
                if let Some(worktree) = project.worktree_for_id(override_id, cx) {
                    return Some(worktree);
                }
            }
        }

        if let Some(repo) = project.active_repository(cx) {
            let repo = repo.read(cx);
            let repo_path = &repo.work_directory_abs_path;

            for worktree in project.visible_worktrees(cx) {
                let worktree_path = worktree.read(cx).abs_path();
                if worktree_path == *repo_path || worktree_path.starts_with(repo_path.as_ref()) {
                    return Some(worktree);
                }
            }
        }

        project.visible_worktrees(cx).next()
    }

    pub fn set_active_worktree_override(
        &mut self,
        worktree_id: WorktreeId,
        cx: &mut Context<Self>,
    ) {
        if let Some(workspace) = self.workspace.upgrade() {
            workspace.update(cx, |workspace, cx| {
                workspace.set_active_worktree_override(Some(worktree_id), cx);
            });
        }
        cx.notify();
    }

    fn clear_active_worktree_override(&mut self, cx: &mut Context<Self>) {
        if let Some(workspace) = self.workspace.upgrade() {
            workspace.update(cx, |workspace, cx| {
                workspace.clear_active_worktree_override(cx);
            });
        }
        cx.notify();
    }

    fn get_repository_for_worktree(
        &self,
        worktree: &Entity<project::Worktree>,
        cx: &App,
    ) -> Option<Entity<project::git_store::Repository>> {
        let project = self.project.read(cx);
        let git_store = project.git_store().read(cx);
        let worktree_path = worktree.read(cx).abs_path();

        git_store
            .repositories()
            .values()
            .filter(|repo| {
                let repo_path = &repo.read(cx).work_directory_abs_path;
                worktree_path == *repo_path || worktree_path.starts_with(repo_path.as_ref())
            })
            .max_by_key(|repo| repo.read(cx).work_directory_abs_path.as_os_str().len())
            .cloned()
    }


    pub fn render_restricted_mode(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let has_restricted_worktrees = TrustedWorktrees::try_get_global(cx)
            .map(|trusted_worktrees| {
                trusted_worktrees
                    .read(cx)
                    .has_restricted_worktrees(&self.project.read(cx).worktree_store(), cx)
            })
            .unwrap_or(false);
        if !has_restricted_worktrees {
            return None;
        }

        let button = Button::new("restricted_mode_trigger", "Restricted Mode")
            .style(ButtonStyle::Tinted(TintColor::Warning))
            .label_size(LabelSize::Small)
            .color(Color::Warning)
            .start_icon(
                Icon::new(IconName::Warning)
                    .size(IconSize::Small)
                    .color(Color::Warning),
            )
            .tooltip(|_, cx| {
                Tooltip::with_meta(
                    "You're in Restricted Mode",
                    Some(&ToggleWorktreeSecurity),
                    "Mark this project as trusted and unlock all features",
                    cx,
                )
            })
            .on_click({
                cx.listener(move |this, _, window, cx| {
                    this.workspace
                        .update(cx, |workspace, cx| {
                            workspace.show_worktree_trust_security_modal(true, window, cx)
                        })
                        .log_err();
                })
            });

        if cfg!(macos_sdk_26) {
            // Make up for Tahoe's traffic light buttons having less spacing around them
            Some(div().child(button).ml_0p5().into_any_element())
        } else {
            Some(button.into_any_element())
        }
    }


    fn render_project_name(
        &self,
        name: Option<SharedString>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let workspace = self.workspace.clone();

        let is_project_selected = name.is_some();

        let display_name = if let Some(ref name) = name {
            util::truncate_and_trailoff(name, MAX_PROJECT_NAME_LENGTH)
        } else {
            "Open Recent Project".to_string()
        };

        let focus_handle = workspace
            .upgrade()
            .map(|w| w.read(cx).focus_handle(cx))
            .unwrap_or_else(|| cx.focus_handle());

        let sibling_workspace_ids: HashSet<WorkspaceId> = self
            .multi_workspace
            .as_ref()
            .and_then(|mw| mw.upgrade())
            .map(|mw| {
                mw.read(cx)
                    .workspaces()
                    .iter()
                    .filter_map(|ws| ws.read(cx).database_id())
                    .collect()
            })
            .unwrap_or_default();

        PopoverMenu::new("recent-projects-menu")
            .menu(move |window, cx| {
                Some(recent_projects::RecentProjects::popover(
                    workspace.clone(),
                    sibling_workspace_ids.clone(),
                    false,
                    focus_handle.clone(),
                    window,
                    cx,
                ))
            })
            .trigger_with_tooltip(
                Button::new("project_name_trigger", display_name)
                    .label_size(LabelSize::Small)
                    .when(self.worktree_count(cx) > 1, |this| {
                        this.end_icon(
                            Icon::new(IconName::ChevronDown)
                                .size(IconSize::XSmall)
                                .color(Color::Muted),
                        )
                    })
                    .selected_style(ButtonStyle::Tinted(TintColor::Accent))
                    .when(!is_project_selected, |s| s.color(Color::Muted)),
                move |_window, cx| {
                    Tooltip::for_action(
                        "Recent Projects",
                        &zed_actions::OpenRecent {
                            create_new_window: false,
                        },
                        cx,
                    )
                },
            )
            .anchor(gpui::Corner::TopLeft)
            .into_any_element()
    }

    fn render_project_branch(
        &self,
        repository: Entity<project::git_store::Repository>,
        linked_worktree_name: Option<SharedString>,
        cx: &mut Context<Self>,
    ) -> Option<impl IntoElement> {
        let workspace = self.workspace.upgrade()?;

        let (branch_name, icon_info) = {
            let repo = repository.read(cx);

            let branch_name = repo
                .branch
                .as_ref()
                .map(|branch| branch.name())
                .map(|name| util::truncate_and_trailoff(name, MAX_BRANCH_NAME_LENGTH))
                .or_else(|| {
                    repo.head_commit.as_ref().map(|commit| {
                        commit
                            .sha
                            .chars()
                            .take(MAX_SHORT_SHA_LENGTH)
                            .collect::<String>()
                    })
                });

            let status = repo.status_summary();
            let tracked = status.index + status.worktree;
            let icon_info = if status.conflict > 0 {
                (IconName::Warning, Color::VersionControlConflict)
            } else if tracked.modified > 0 {
                (IconName::SquareDot, Color::VersionControlModified)
            } else if tracked.added > 0 || status.untracked > 0 {
                (IconName::SquarePlus, Color::VersionControlAdded)
            } else if tracked.deleted > 0 {
                (IconName::SquareMinus, Color::VersionControlDeleted)
            } else {
                (IconName::GitBranch, Color::Muted)
            };

            (branch_name, icon_info)
        };

        let branch_name = branch_name?;
        let settings = TitleBarSettings::get_global(cx);
        let effective_repository = Some(repository);

        Some(
            PopoverMenu::new("branch-menu")
                .menu(move |window, cx| {
                    Some(git_ui::git_picker::popover(
                        workspace.downgrade(),
                        effective_repository.clone(),
                        git_ui::git_picker::GitPickerTab::Branches,
                        gpui::rems(34.),
                        window,
                        cx,
                    ))
                })
                .trigger_with_tooltip(
                    ButtonLike::new("project_branch_trigger")
                        .selected_style(ButtonStyle::Tinted(TintColor::Accent))
                        .child(
                            h_flex()
                                .gap_0p5()
                                .when(settings.show_branch_icon, |this| {
                                    let (icon, icon_color) = icon_info;
                                    this.child(
                                        Icon::new(icon).size(IconSize::XSmall).color(icon_color),
                                    )
                                })
                                .when_some(linked_worktree_name.as_ref(), |this, worktree_name| {
                                    this.child(
                                        Label::new(worktree_name)
                                            .size(LabelSize::Small)
                                            .color(Color::Muted),
                                    )
                                    .child(
                                        Label::new("/").size(LabelSize::Small).color(
                                            Color::Custom(
                                                cx.theme().colors().text_muted.opacity(0.4),
                                            ),
                                        ),
                                    )
                                })
                                .child(
                                    Label::new(branch_name)
                                        .size(LabelSize::Small)
                                        .color(Color::Muted),
                                ),
                        ),
                    move |_window, cx| {
                        Tooltip::with_meta(
                            "Git Switcher",
                            Some(&zed_actions::git::Branch),
                            "Worktrees, Branches, and Stashes",
                            cx,
                        )
                    },
                )
                .anchor(gpui::Corner::TopLeft),
        )
    }

    fn window_activation_changed(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.workspace
            .update(cx, |workspace, cx| {
                workspace.update_active_view_for_followers(_window, cx);
            })
            .ok();
    }

}
