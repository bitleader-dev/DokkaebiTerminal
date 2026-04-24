use std::path::PathBuf;
use std::sync::Arc;

use fuzzy::StringMatchCandidate;
use git::repository::Worktree as GitWorktree;
use gpui::{
    AnyElement, App, Context, DismissEvent, Entity, EventEmitter, FocusHandle, Focusable,
    IntoElement, ParentElement, Render, SharedString, Styled, Subscription, Task, Window, rems,
};
use picker::{Picker, PickerDelegate, PickerEditorPosition};
use project::Project;
use project::git_store::RepositoryEvent;
use ui::{ListItem, ListItemSpacing, Tooltip, prelude::*};
use util::ResultExt as _;

use crate::{CreateWorktree, NewWorktreeBranchTarget};

pub(crate) struct ThreadWorktreePicker {
    picker: Entity<Picker<ThreadWorktreePickerDelegate>>,
    focus_handle: FocusHandle,
    _subscriptions: Vec<Subscription>,
}

impl ThreadWorktreePicker {
    pub fn new(project: Entity<Project>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let has_multiple_repositories = project.read(cx).repositories(cx).len() > 1;

        let current_branch_name = project.read(cx).active_repository(cx).and_then(|repo| {
            repo.read(cx)
                .branch
                .as_ref()
                .map(|branch| branch.name().to_string())
        });

        let repository = if has_multiple_repositories {
            None
        } else {
            project.read(cx).active_repository(cx)
        };

        // Fetch worktrees from the git backend (includes main + all linked)
        let all_worktrees_request = repository
            .clone()
            .map(|repo| repo.update(cx, |repo, _| repo.worktrees()));

        let default_branch_request = repository
            .clone()
            .map(|repo| repo.update(cx, |repo, _| repo.default_branch(false)));

        let initial_matches = vec![ThreadWorktreeEntry::CreateFromCurrentBranch];

        let delegate = ThreadWorktreePickerDelegate {
            matches: initial_matches,
            all_worktrees: Vec::new(),
            selected_index: 0,
            project,
            current_branch_name,
            default_branch_name: None,
            has_multiple_repositories,
        };

        let picker = cx.new(|cx| {
            Picker::list(delegate, window, cx)
                .list_measure_all()
                .modal(false)
                .max_height(Some(rems(20.).into()))
        });

        let mut subscriptions = Vec::new();

        // Fetch worktrees and default branch asynchronously
        {
            let picker_handle = picker.downgrade();
            cx.spawn_in(window, async move |_this, cx| {
                let all_worktrees: Vec<_> = match all_worktrees_request {
                    Some(req) => match req.await {
                        Ok(Ok(worktrees)) => {
                            worktrees.into_iter().filter(|wt| !wt.is_bare).collect()
                        }
                        Ok(Err(err)) => {
                            log::warn!("ThreadWorktreePicker: git worktree list failed: {err}");
                            return anyhow::Ok(());
                        }
                        Err(_) => {
                            log::warn!("ThreadWorktreePicker: worktree request was cancelled");
                            return anyhow::Ok(());
                        }
                    },
                    None => Vec::new(),
                };

                let default_branch = match default_branch_request {
                    Some(req) => req.await.ok().and_then(Result::ok).flatten(),
                    None => None,
                };

                picker_handle.update_in(cx, |picker, window, cx| {
                    picker.delegate.all_worktrees = all_worktrees;
                    picker.delegate.default_branch_name =
                        default_branch.map(|branch| branch.to_string());
                    picker.refresh(window, cx);
                })?;

                anyhow::Ok(())
            })
            .detach_and_log_err(cx);
        }

        // Subscribe to repository events to live-update the worktree list
        if let Some(repo) = &repository {
            let picker_entity = picker.downgrade();
            subscriptions.push(cx.subscribe_in(
                repo,
                window,
                move |_this, repo, event: &RepositoryEvent, window, cx| {
                    if matches!(event, RepositoryEvent::GitWorktreeListChanged) {
                        let worktrees_request = repo.update(cx, |repo, _| repo.worktrees());
                        let picker = picker_entity.clone();
                        cx.spawn_in(window, async move |_, cx| {
                            let all_worktrees: Vec<_> = worktrees_request
                                .await??
                                .into_iter()
                                .filter(|wt| !wt.is_bare)
                                .collect();
                            picker.update_in(cx, |picker, window, cx| {
                                picker.delegate.all_worktrees = all_worktrees;
                                picker.refresh(window, cx);
                            })?;
                            anyhow::Ok(())
                        })
                        .detach_and_log_err(cx);
                    }
                },
            ));
        }

        subscriptions.push(cx.subscribe(&picker, |_, _, _, cx| {
            cx.emit(DismissEvent);
        }));

        Self {
            focus_handle: picker.focus_handle(cx),
            picker,
            _subscriptions: subscriptions,
        }
    }
}

impl Focusable for ThreadWorktreePicker {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl EventEmitter<DismissEvent> for ThreadWorktreePicker {}

impl Render for ThreadWorktreePicker {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .w(rems(34.))
            .elevation_3(cx)
            .child(self.picker.clone())
            .on_mouse_down_out(cx.listener(|_, _, _, cx| {
                cx.emit(DismissEvent);
            }))
    }
}

#[derive(Clone)]
enum ThreadWorktreeEntry {
    CreateFromCurrentBranch,
    CreateFromDefaultBranch {
        default_branch_name: String,
    },
    CreateNamed {
        name: String,
        /// When Some, create from this branch name (e.g. "main"). When None, create from current branch.
        from_branch: Option<String>,
        disabled_reason: Option<String>,
    },
}

pub(crate) struct ThreadWorktreePickerDelegate {
    matches: Vec<ThreadWorktreeEntry>,
    all_worktrees: Vec<GitWorktree>,
    selected_index: usize,
    project: Entity<Project>,
    current_branch_name: Option<String>,
    default_branch_name: Option<String>,
    has_multiple_repositories: bool,
}

impl ThreadWorktreePickerDelegate {
    fn build_fixed_entries(&self) -> Vec<ThreadWorktreeEntry> {
        let mut entries = Vec::new();

        entries.push(ThreadWorktreeEntry::CreateFromCurrentBranch);

        if !self.has_multiple_repositories {
            if let Some(ref default_branch) = self.default_branch_name {
                let is_different = self
                    .current_branch_name
                    .as_ref()
                    .is_none_or(|current| current != default_branch);
                if is_different {
                    entries.push(ThreadWorktreeEntry::CreateFromDefaultBranch {
                        default_branch_name: default_branch.clone(),
                    });
                }
            }
        }

        entries
    }

    fn sync_selected_index(&mut self, has_query: bool) {
        if !has_query {
            return;
        }

        // 쿼리 입력 시 가장 적합한 신규 worktree 생성 항목을 선택한다.
        if let Some(index) = self
            .matches
            .iter()
            .position(|entry| matches!(entry, ThreadWorktreeEntry::CreateNamed { .. }))
        {
            self.selected_index = index;
        } else {
            self.selected_index = 0;
        }
    }
}

impl PickerDelegate for ThreadWorktreePickerDelegate {
    type ListItem = AnyElement;

    fn placeholder_text(&self, _window: &mut Window, _cx: &mut App) -> Arc<str> {
        "Select a worktree for this thread…".into()
    }

    fn editor_position(&self) -> PickerEditorPosition {
        PickerEditorPosition::Start
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

    fn can_select(&self, ix: usize, _window: &mut Window, _cx: &mut Context<Picker<Self>>) -> bool {
        self.matches.get(ix).is_some()
    }

    fn update_matches(
        &mut self,
        query: String,
        _window: &mut Window,
        _cx: &mut Context<Picker<Self>>,
    ) -> Task<()> {
        if query.is_empty() {
            self.matches = self.build_fixed_entries();
            self.sync_selected_index(false);
            return Task::ready(());
        }

        let normalized_query = query.replace(' ', "-");
        let main_worktree_path = self
            .all_worktrees
            .iter()
            .find(|wt| wt.is_main)
            .map(|wt| wt.path.clone());
        let has_named_worktree = self.all_worktrees.iter().any(|worktree| {
            worktree.directory_name(main_worktree_path.as_deref()) == normalized_query
        });
        let create_named_disabled_reason: Option<String> = if self.has_multiple_repositories {
            Some("Cannot create a named worktree in a project with multiple repositories".into())
        } else if has_named_worktree {
            Some("A worktree with this name already exists".into())
        } else {
            None
        };

        let show_default_branch_create = !self.has_multiple_repositories
            && self.default_branch_name.as_ref().is_some_and(|default| {
                self.current_branch_name
                    .as_ref()
                    .is_none_or(|current| current != default)
            });

        let mut new_matches: Vec<ThreadWorktreeEntry> = Vec::new();
        new_matches.push(ThreadWorktreeEntry::CreateNamed {
            name: normalized_query.clone(),
            from_branch: None,
            disabled_reason: create_named_disabled_reason.clone(),
        });
        if show_default_branch_create {
            if let Some(default_branch) = self.default_branch_name.clone() {
                new_matches.push(ThreadWorktreeEntry::CreateNamed {
                    name: normalized_query,
                    from_branch: Some(default_branch),
                    disabled_reason: create_named_disabled_reason,
                });
            }
        }

        self.matches = new_matches;
        self.sync_selected_index(true);
        Task::ready(())
    }

    fn confirm(&mut self, _secondary: bool, window: &mut Window, cx: &mut Context<Picker<Self>>) {
        let Some(entry) = self.matches.get(self.selected_index) else {
            return;
        };

        match entry {
            ThreadWorktreeEntry::CreateFromCurrentBranch => {
                window.dispatch_action(
                    Box::new(CreateWorktree {
                        worktree_name: None,
                        branch_target: NewWorktreeBranchTarget::CurrentBranch,
                    }),
                    cx,
                );
            }

            ThreadWorktreeEntry::CreateFromDefaultBranch {
                default_branch_name,
            } => {
                window.dispatch_action(
                    Box::new(CreateWorktree {
                        worktree_name: None,
                        branch_target: NewWorktreeBranchTarget::ExistingBranch {
                            name: default_branch_name.clone(),
                        },
                    }),
                    cx,
                );
            }

            ThreadWorktreeEntry::CreateNamed {
                name,
                from_branch,
                disabled_reason: None,
            } => {
                let branch_target = match from_branch {
                    Some(branch) => NewWorktreeBranchTarget::ExistingBranch {
                        name: branch.clone(),
                    },
                    None => NewWorktreeBranchTarget::CurrentBranch,
                };
                window.dispatch_action(
                    Box::new(CreateWorktree {
                        worktree_name: Some(name.clone()),
                        branch_target,
                    }),
                    cx,
                );
            }

            ThreadWorktreeEntry::CreateNamed {
                disabled_reason: Some(_),
                ..
            } => {
                return;
            }
        }

        cx.emit(DismissEvent);
    }

    fn dismissed(&mut self, _window: &mut Window, _cx: &mut Context<Picker<Self>>) {}

    fn render_match(
        &self,
        ix: usize,
        selected: bool,
        _window: &mut Window,
        cx: &mut Context<Picker<Self>>,
    ) -> Option<Self::ListItem> {
        let entry = self.matches.get(ix)?;
        let project = self.project.read(cx);
        let is_create_disabled = project.repositories(cx).is_empty() || project.is_via_collab();

        let no_git_reason: SharedString = "Requires a Git repository in the project".into();

        let create_new_list_item = |id: SharedString,
                                    label: SharedString,
                                    disabled_tooltip: Option<SharedString>,
                                    selected: bool| {
            let is_disabled = disabled_tooltip.is_some();
            ListItem::new(id)
                .inset(true)
                .spacing(ListItemSpacing::Sparse)
                .toggle_state(selected)
                .child(
                    h_flex()
                        .w_full()
                        .gap_2p5()
                        .child(
                            Icon::new(IconName::Plus)
                                .map(|this| {
                                    if is_disabled {
                                        this.color(Color::Disabled)
                                    } else {
                                        this.color(Color::Muted)
                                    }
                                })
                                .size(IconSize::Small),
                        )
                        .child(
                            Label::new(label).when(is_disabled, |this| this.color(Color::Disabled)),
                        ),
                )
                .when_some(disabled_tooltip, |this, reason| {
                    this.tooltip(Tooltip::text(reason))
                })
                .into_any_element()
        };

        match entry {
            ThreadWorktreeEntry::CreateFromCurrentBranch => {
                let branch_label = if self.has_multiple_repositories {
                    "current branches".to_string()
                } else {
                    self.current_branch_name
                        .clone()
                        .unwrap_or_else(|| "HEAD".to_string())
                };

                let label = format!("Create new worktree based on {branch_label}");

                let disabled_tooltip = is_create_disabled.then(|| no_git_reason.clone());

                let item = create_new_list_item(
                    "create-from-current".to_string().into(),
                    label.into(),
                    disabled_tooltip,
                    selected,
                );

                Some(item.into_any_element())
            }

            ThreadWorktreeEntry::CreateFromDefaultBranch {
                default_branch_name,
            } => {
                let label = format!("Create new worktree based on {default_branch_name}");

                let disabled_tooltip = is_create_disabled.then(|| no_git_reason.clone());

                let item = create_new_list_item(
                    "create-from-main".to_string().into(),
                    label.into(),
                    disabled_tooltip,
                    selected,
                );

                Some(item.into_any_element())
            }

            ThreadWorktreeEntry::CreateNamed {
                name,
                from_branch,
                disabled_reason,
            } => {
                let branch_label = from_branch
                    .as_deref()
                    .unwrap_or(self.current_branch_name.as_deref().unwrap_or("HEAD"));
                let label = format!("Create \"{name}\" based on {branch_label}");
                let element_id = match from_branch {
                    Some(branch) => format!("create-named-from-{branch}"),
                    None => "create-named-from-current".to_string(),
                };

                let item = create_new_list_item(
                    element_id.into(),
                    label.into(),
                    disabled_reason.clone().map(SharedString::from),
                    selected,
                );

                Some(item.into_any_element())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fs::FakeFs;
    use gpui::TestAppContext;
    use project::Project;
    use settings::SettingsStore;

    fn init_test(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let settings_store = SettingsStore::test(cx);
            cx.set_global(settings_store);
            theme_settings::init(theme::LoadThemes::JustBase, cx);
            editor::init(cx);
            release_channel::init("0.0.0".parse().unwrap(), cx);
            crate::agent_panel::init(cx);
        });
    }

    fn make_worktree(path: &str, branch: &str, is_main: bool) -> GitWorktree {
        GitWorktree {
            path: PathBuf::from(path),
            ref_name: Some(format!("refs/heads/{branch}").into()),
            sha: "abc1234".into(),
            is_main,
            is_bare: false,
        }
    }

    fn build_delegate(
        project: Entity<Project>,
        all_worktrees: Vec<GitWorktree>,
        current_branch_name: Option<String>,
        default_branch_name: Option<String>,
        has_multiple_repositories: bool,
    ) -> ThreadWorktreePickerDelegate {
        ThreadWorktreePickerDelegate {
            matches: vec![ThreadWorktreeEntry::CreateFromCurrentBranch],
            all_worktrees,
            selected_index: 0,
            project,
            current_branch_name,
            default_branch_name,
            has_multiple_repositories,
        }
    }

    fn entry_names(delegate: &ThreadWorktreePickerDelegate) -> Vec<String> {
        delegate
            .matches
            .iter()
            .map(|entry| match entry {
                ThreadWorktreeEntry::CreateFromCurrentBranch => {
                    "CreateFromCurrentBranch".to_string()
                }
                ThreadWorktreeEntry::CreateFromDefaultBranch {
                    default_branch_name,
                } => format!("CreateFromDefaultBranch({default_branch_name})"),
                ThreadWorktreeEntry::CreateNamed {
                    name,
                    from_branch,
                    disabled_reason,
                } => {
                    let branch = from_branch
                        .as_deref()
                        .map(|b| format!("from {b}"))
                        .unwrap_or_else(|| "from current".to_string());
                    if disabled_reason.is_some() {
                        format!("CreateNamed({name}, {branch}, disabled)")
                    } else {
                        format!("CreateNamed({name}, {branch})")
                    }
                }
            })
            .collect()
    }

    type PickerWindow = gpui::WindowHandle<Picker<ThreadWorktreePickerDelegate>>;

    async fn make_picker(
        cx: &mut TestAppContext,
        all_worktrees: Vec<GitWorktree>,
        current_branch_name: Option<String>,
        default_branch_name: Option<String>,
        has_multiple_repositories: bool,
    ) -> PickerWindow {
        let fs = FakeFs::new(cx.executor());
        let project = Project::test(fs, [], cx).await;

        cx.add_window(|window, cx| {
            let delegate = build_delegate(
                project,
                all_worktrees,
                current_branch_name,
                default_branch_name,
                has_multiple_repositories,
            );
            Picker::list(delegate, window, cx)
                .list_measure_all()
                .modal(false)
        })
    }

    #[gpui::test]
    async fn test_empty_query_entries(cx: &mut TestAppContext) {
        init_test(cx);

        // 현재 브랜치와 기본 브랜치가 모두 `main` 이면 CreateFromCurrentBranch 만 노출된다.
        let worktrees = vec![
            make_worktree("/repo", "main", true),
            make_worktree("/repo-feature", "feature", false),
            make_worktree("/repo-bugfix", "bugfix", false),
        ];

        let picker = make_picker(
            cx,
            worktrees,
            Some("main".into()),
            Some("main".into()),
            false,
        )
        .await;

        picker
            .update(cx, |picker, window, cx| picker.refresh(window, cx))
            .unwrap();
        cx.run_until_parked();

        let names = picker
            .read_with(cx, |picker, _| entry_names(&picker.delegate))
            .unwrap();

        assert_eq!(names, vec!["CreateFromCurrentBranch"]);

        // 현재 브랜치와 기본 브랜치가 다르면 CreateFromDefaultBranch 가 추가된다.
        picker
            .update(cx, |picker, _window, cx| {
                picker.delegate.current_branch_name = Some("feature".into());
                picker.delegate.default_branch_name = Some("main".into());
                cx.notify();
            })
            .unwrap();
        picker
            .update(cx, |picker, window, cx| picker.refresh(window, cx))
            .unwrap();
        cx.run_until_parked();

        let names = picker
            .read_with(cx, |picker, _| entry_names(&picker.delegate))
            .unwrap();

        assert!(names.contains(&"CreateFromDefaultBranch(main)".to_string()));
    }

    #[gpui::test]
    async fn test_query_filtering_and_create_entries(cx: &mut TestAppContext) {
        init_test(cx);

        let picker = make_picker(
            cx,
            vec![
                make_worktree("/repo", "main", true),
                make_worktree("/repo-feature", "feature", false),
                make_worktree("/repo-bugfix", "bugfix", false),
                make_worktree("/my-worktree", "experiment", false),
            ],
            Some("dev".into()),
            Some("main".into()),
            false,
        )
        .await;

        // 부분 일치 쿼리에서는 현재/기본 브랜치 양쪽으로 CreateNamed 를 제안한다.
        picker
            .update(cx, |picker, window, cx| {
                picker.set_query("feat", window, cx)
            })
            .unwrap();
        cx.run_until_parked();

        let names = picker
            .read_with(cx, |picker, _| entry_names(&picker.delegate))
            .unwrap();
        assert!(
            names.contains(&"CreateNamed(feat, from current)".to_string()),
            "should offer to create from current branch, got: {names:?}"
        );
        assert!(
            names.contains(&"CreateNamed(feat, from main)".to_string()),
            "should offer to create from default branch, got: {names:?}"
        );

        // Exact match: both create entries appear but are disabled.
        picker
            .update(cx, |picker, window, cx| {
                picker.set_query("repo-feature", window, cx)
            })
            .unwrap();
        cx.run_until_parked();

        let names = picker
            .read_with(cx, |picker, _| entry_names(&picker.delegate))
            .unwrap();
        assert!(
            names.contains(&"CreateNamed(repo-feature, from current, disabled)".to_string()),
            "exact name match should show disabled create entries, got: {names:?}"
        );

        // Spaces are normalized to hyphens: "my worktree" matches "my-worktree".
        picker
            .update(cx, |picker, window, cx| {
                picker.set_query("my worktree", window, cx)
            })
            .unwrap();
        cx.run_until_parked();

        let names = picker
            .read_with(cx, |picker, _| entry_names(&picker.delegate))
            .unwrap();
        assert!(
            names.contains(&"CreateNamed(my-worktree, from current, disabled)".to_string()),
            "spaces should normalize to hyphens and detect existing worktree, got: {names:?}"
        );
    }

    #[gpui::test]
    async fn test_multi_repo_hides_worktrees_and_disables_create_named(cx: &mut TestAppContext) {
        init_test(cx);

        let picker = make_picker(
            cx,
            vec![
                make_worktree("/repo", "main", true),
                make_worktree("/repo-feature", "feature", false),
            ],
            Some("main".into()),
            Some("main".into()),
            true,
        )
        .await;

        picker
            .update(cx, |picker, window, cx| picker.refresh(window, cx))
            .unwrap();
        cx.run_until_parked();

        let names = picker
            .read_with(cx, |picker, _| entry_names(&picker.delegate))
            .unwrap();
        assert_eq!(names, vec!["CreateFromCurrentBranch"]);

        picker
            .update(cx, |picker, window, cx| {
                picker.set_query("new-thing", window, cx)
            })
            .unwrap();
        cx.run_until_parked();

        let names = picker
            .read_with(cx, |picker, _| entry_names(&picker.delegate))
            .unwrap();
        assert!(
            names.contains(&"CreateNamed(new-thing, from current, disabled)".to_string()),
            "multi-repo should disable create named, got: {names:?}"
        );
    }
}
